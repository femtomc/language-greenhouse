use std::collections::BTreeMap;

// A simple language, but quite unbreakable.

type Value = i64;
type Name = String;

enum Expr {
    Value(Value),
    EAdd(Box<Expr>, Box<Expr>),
    ESub(Box<Expr>, Box<Expr>),
    EMul(Box<Expr>, Box<Expr>),
    EAssign(Name, Box<Expr>),
    EVar(Name),
}

pub use Expr::*;

// An interpreter with eval.

struct Interpreter {
    env: BTreeMap<Name, Value>,
}

impl Interpreter {
    pub fn eval(&mut self, e: Expr) -> Value {
        match e {
            Value(v) => v,
            EVar(n) => self.env.get(&n).unwrap().clone(),
            EAdd(e1, e2) => self.eval(*e1) + self.eval(*e2),
            ESub(e1, e2) => self.eval(*e1) - self.eval(*e2),
            EMul(e1, e2) => self.eval(*e1) * self.eval(*e2),
            EAssign(n, e) => {
                let v = self.eval(*e);
                self.env.insert(n, v);
                v.clone()
            }
        }
    }
}
