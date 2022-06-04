use color_eyre::{eyre::bail, eyre::eyre, Report};
use std::collections::BTreeMap;

// A simple language, but quite unbreakable.

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

// An interpreter with eval.

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

// Parser

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

// REPL

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
