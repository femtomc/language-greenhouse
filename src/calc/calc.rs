use color_eyre::{eyre::eyre, Report};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataContext, Linkage, Module};
use std::collections::BTreeMap;

/////
///// A simple language, but quite unbreakable.
/////

type Value = i64;
type Name = String;

#[derive(Debug)]
pub enum Expr {
    Value(Value),
    EVar(Name),
    ENeg(Box<Expr>),
    EAdd(Box<Expr>, Box<Expr>),
    ESub(Box<Expr>, Box<Expr>),
    EMul(Box<Expr>, Box<Expr>),
    ELet(Name, Box<Expr>, Box<Expr>),
}

pub use Expr::*;

/////
///// An interpreter with eval.
/////

pub struct Interpreter {
    env: BTreeMap<Name, Value>,
}

impl Interpreter {
    pub fn new() -> Self {
        return Interpreter {
            env: BTreeMap::new(),
        };
    }
    pub fn eval(&mut self, e: Expr) -> Result<Value, Report> {
        Ok(match e {
            Value(v) => v,
            EVar(n) => self.env.get(&n).unwrap().clone(),
            ENeg(e) => -self.eval(*e)?,
            EAdd(e1, e2) => self.eval(*e1)? + self.eval(*e2)?,
            ESub(e1, e2) => self.eval(*e1)? - self.eval(*e2)?,
            EMul(e1, e2) => self.eval(*e1)? * self.eval(*e2)?,
            ELet(n, e1, e2) => {
                let v = self.eval(*e1)?;
                self.env.insert(n, v);
                self.eval(*e2)?
            }
        })
    }
}

/////
///// Parser
/////

use chumsky::prelude::*;

fn parser() -> impl Parser<char, Expr, Error = Simple<char>> {
    let ident = text::ident().padded();

    let expr = recursive(|expr| {
        let int = text::int(10)
            .map(|s: String| Expr::Value(s.parse().unwrap()))
            .padded();

        let atom = int
            .or(expr.delimited_by(just('('), just(')')))
            .or(ident.map(Expr::EVar));

        let op = |c| just(c).padded();

        let unary = op('-')
            .repeated()
            .then(atom)
            .foldr(|_op, rhs| Expr::ENeg(Box::new(rhs)));

        let product = unary
            .clone()
            .then(
                op('*')
                    .to(Expr::EMul as fn(_, _) -> _)
                    .then(unary)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));

        let sum = product
            .clone()
            .then(
                op('+')
                    .to(Expr::EAdd as fn(_, _) -> _)
                    .or(op('-').to(Expr::ESub as fn(_, _) -> _))
                    .then(product)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));

        sum
    });

    let decl = recursive(|decl| {
        let r#let = text::keyword("let")
            .ignore_then(ident)
            .then_ignore(just('='))
            .then(expr.clone())
            .then_ignore(just(';'))
            .then(decl.clone())
            .map(|((name, rhs), then)| Expr::ELet(name, Box::new(rhs), Box::new(then)));

        r#let.or(expr).padded()
    });

    decl.then_ignore(end())
}

pub fn eval(src: &str) -> Result<Value, Report> {
    let mut interp = Interpreter::new();
    match parser().parse(src) {
        Ok(ast) => interp.eval(ast),
        Err(parse_errs) => Err(eyre!(parse_errs
            .into_iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .concat())),
    }
}

/////
///// Staging the interpreter
/////

// This is a self-contained compiler -- using Cranelift
// for code generation.

pub struct StagedInterpreter {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    data_ctx: DataContext,
    module: JITModule,
}

impl StagedInterpreter {
    pub fn new() -> Self {
        let builder = JITBuilder::new(cranelift_module::default_libcall_names()).unwrap();
        let module = JITModule::new(builder);
        let builder_context = FunctionBuilderContext::new();
        let ctx = module.make_context();

        StagedInterpreter {
            builder_context,
            ctx,
            data_ctx: DataContext::new(),
            module,
        }
    }

    pub unsafe fn eval(mut self, e: Expr) -> Result<Value, Report> {
        let int = self.module.target_config().pointer_type();
        self.ctx.func.signature.returns.push(AbiParam::new(int));
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);
        let mut translator = FunctionTranslator {
            index: 0,
            int,
            env: BTreeMap::new(),
            builder,
        };

        // The interpreter creates a `FunctionTranslator` -- which holds
        // the state required to do code generation.
        // Now, hand off `eval` to the translator.
        let v = translator.eval(e)?;
        translator.builder.ins().return_(&[v]);
        translator.builder.finalize();
        println!("Translated:\n{}", self.ctx.func);
        let id = self
            .module
            .declare_function("main", Linkage::Export, &self.ctx.func.signature)
            .map_err(|e| eyre!(e.to_string()))?;
        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|e| eyre!(e.to_string()))?;
        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions();
        let code_ptr = self.module.get_finalized_function(id);

        // Cast the raw pointer to a typed function pointer. This is unsafe, because
        // this is the critical point where you have to trust that the generated code
        let code_fn = std::mem::transmute::<_, fn() -> Value>(code_ptr);
        // is safe to be called.
        // And now we can call it!
        Ok(code_fn())
    }
}

pub struct FunctionTranslator<'a> {
    index: usize,
    int: types::Type,
    env: BTreeMap<Name, Variable>,
    builder: FunctionBuilder<'a>,
}

impl<'a> FunctionTranslator<'a> {
    // Now here, instead of evaluation -- we do code generation.
    pub fn eval(&mut self, e: Expr) -> Result<cranelift::prelude::Value, Report> {
        Ok(match e {
            Value(v) => self.builder.ins().iconst(self.int, v),
            EVar(n) => {
                let variable = self.env.get(&n).unwrap();
                self.builder.use_var(*variable)
            }
            ENeg(e) => {
                let v = self.eval(*e)?;
                self.builder.ins().ineg(v)
            }
            EAdd(e1, e2) => {
                let lhs = self.eval(*e1)?;
                let rhs = self.eval(*e2)?;
                self.builder.ins().iadd(lhs, rhs)
            }
            ESub(e1, e2) => {
                let lhs = self.eval(*e1)?;
                let rhs = self.eval(*e2)?;
                self.builder.ins().isub(lhs, rhs)
            }
            EMul(e1, e2) => {
                let lhs = self.eval(*e1)?;
                let rhs = self.eval(*e2)?;
                self.builder.ins().imul(lhs, rhs)
            }

            // Notice here how nested let-bindings
            // shadow -- this falls naturally out of codegen
            // along the path that the interpreter walks.
            ELet(n, e1, e2) => {
                let v = self.eval(*e1)?;
                let var = Variable::new(self.index);
                self.index += 1;
                self.env.insert(n, var);
                self.builder.declare_var(var, self.int);
                self.builder.def_var(var, v);
                self.eval(*e2)?
            }
        })
    }
}

pub fn eval_staged(src: &str) -> Result<Value, Report> {
    let interp = StagedInterpreter::new();
    match parser().parse(src) {
        Ok(ast) => unsafe { interp.eval(ast) },
        Err(parse_errs) => Err(eyre!(parse_errs
            .into_iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .concat())),
    }
}
