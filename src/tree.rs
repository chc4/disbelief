#![feature(trait_alias)]

use core::fmt::{Debug, Formatter};
impl Debug for Value {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Value::Number(u) => write!(fmt, "{}", u),
            Value::Coroutine(c) => write!(fmt, "<coroutine>"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(usize),
    Add(Box<Expr>, Box<Expr>),
    Suspend,
    Resume(Box<Expr>, Box<Expr>),
}

trait Cont = FnOnce(Value) -> Value;

pub enum Value {
    Number(usize),
    Coroutine(Box<dyn Cont>),
}


pub fn eval(e: &Expr, cont: Box<dyn Cont>) -> Value {
    //println!("{:?}", e);
    match e {
        Expr::Number(u) => cont(Value::Number(*u)),
        Expr::Add(l, r) => {
            let r = r.clone();
            cont(eval(l, Box::new(move |l_val| {
                if let Value::Number(l_num) = l_val {
                    eval(&r, Box::new(move |r_val| {
                        if let Value::Number(r_num) = r_val {
                            Value::Number(l_num + r_num)
                        } else {
                            panic!("can't add non-number right operand");
                        }
                    }))
                } else {
                    panic!("can't add non-number left operand");
                }
            })))
        },
        Expr::Suspend => {
            Value::Coroutine(Box::new(|v| cont(v)))
        },
        Expr::Resume(c, r) => {
            let r = r.clone();
            cont(eval(c, Box::new(move |coro_val| {
                if let Value::Coroutine(coro) = coro_val {
                    eval(&r, Box::new(move |r_val| {
                        coro(r_val)
                    }))
                } else {
                    panic!("can't resume non-coroutine");
                }
            })))
        }
    }
}


fn main() {
    println!("{:?}", eval(&Expr::Add(
        Box::new(Expr::Number(1)),
        Box::new(Expr::Number(2))),
        Box::new(|v| v)));
    println!("{:?}", eval(&Expr::Resume(
        Box::new(Expr::Add(
            Box::new(Expr::Suspend),
            Box::new(Expr::Number(2)))),
        Box::new(Expr::Number(1))),
            Box::new(|v| v)));
    println!("{:?}", eval(&Expr::Resume(
        Box::new(Expr::Add(
            Box::new(Expr::Number(1)),
            Box::new(Expr::Suspend))),
        Box::new(Expr::Number(2))),
            Box::new(|v| v)));
}
