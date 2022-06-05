use color_eyre::{eyre::bail, eyre::eyre, Report};
use std::collections::BTreeMap;

/////
///// Lambda?
/////

#[derive(Debug, Clone)]
pub enum Value {
    VInt(i64),
    VFunc(Box<Expr>),
}

type Name = String;

#[derive(Debug, Clone)]
pub enum Expr {
    EVal(Value),
    EVar(Name),
    ENeg(Box<Expr>),
    EAdd(Box<Expr>, Box<Expr>),
    ESub(Box<Expr>, Box<Expr>),
    EMul(Box<Expr>, Box<Expr>),
    EAbs(Box<Expr>, Box<Expr>),
    EApp(Box<Expr>, Box<Expr>),
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
            EVal(v) => v,
            EVar(n) => self.env.get(&n).unwrap().clone(),
            ENeg(e) => match self.eval(*e)? {
                Value::VInt(v) => Value::VInt(-v),
                _ => bail!("Expected a value of type Int."),
            },
            EAdd(e1, e2) => match self.eval(*e1)? {
                Value::VInt(v1) => match self.eval(*e2)? {
                    Value::VInt(v2) => Value::VInt(v1 + v2),
                    _ => bail!("Expected a value of type Int."),
                },
                _ => bail!("Expected a value of type Int."),
            },
            ESub(e1, e2) => match self.eval(*e1)? {
                Value::VInt(v1) => match self.eval(*e2)? {
                    Value::VInt(v2) => Value::VInt(v1 - v2),
                    _ => bail!("Expected a value of type Int."),
                },
                _ => bail!("Expected a value of type Int."),
            },
            EMul(e1, e2) => match self.eval(*e1)? {
                Value::VInt(v1) => match self.eval(*e2)? {
                    Value::VInt(v2) => Value::VInt(v1 * v2),
                    _ => bail!("Expected a value of type Int."),
                },
                _ => bail!("Expected a value of type Int."),
            },
            EAbs(n, e) => Value::VFunc(Box::new(EAbs(n, e))),
            EApp(e1, e2) => match self.eval(*e1)? {
                Value::VFunc(e) => match *e {
                    EAbs(n, body) => {
                        let reduced = self.eval(*e2)?;
                        match *n {
                            EVar(name) => {
                                self.env.insert(name.to_string(), reduced);
                                let result = self.eval(*body)?;
                                self.env.remove(&name);
                                result
                            }
                            _ => bail!("Expected a variable name."),
                        }
                    }
                    _ => bail!("Expected a value of type Func."),
                },
                _ => bail!("Expected a value of type Func."),
            },
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
            .map(|s: String| Expr::EVal(Value::VInt(s.parse().unwrap())))
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

        let func = sum
            .clone()
            .then(op('.').to(Expr::EAbs as fn(_, _) -> _).then(sum).repeated())
            .foldl(|lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)));

        func
    });

    let decl = recursive(|decl| {
        let r#call = ident
            .map(Expr::EVar)
            .then_ignore(just('('))
            .then(expr.clone())
            .then_ignore(just(')'))
            .map(|(name, arg)| Expr::EApp(Box::new(name), Box::new(arg)));

        let r#let = text::keyword("let")
            .ignore_then(ident)
            .then_ignore(just('='))
            .then(expr.clone())
            .then_ignore(just(';'))
            .then(decl.clone())
            .map(|((name, rhs), then)| Expr::ELet(name, Box::new(rhs), Box::new(then)));

        r#let.or(r#call).or(expr).padded()
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
