#![feature(trait_alias)]
pub mod tree;
use std::rc::Rc;

use std::fmt::{Debug, Formatter};
impl Debug for Value {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Value::Number(u) => write!(fmt, "{}", u),
            Value::Coroutine(c) => write!(fmt, "<coroutine {:p}>", c),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(usize),
    Add(Rc<Expr>, Rc<Expr>),
    Suspend,
    Resume(Rc<Expr>, Rc<Expr>),
}

pub type Env = ();
trait Compile = Fn(()) -> Value;
trait Cont = Fn(Box<dyn Compile>) -> Box<dyn Compile>;

pub enum Value {
    Number(usize),
    Coroutine(Rc<dyn Cont>),
}

pub fn compile<'a>(e: &'a Expr, cont: Rc<dyn Cont>) -> Box<dyn Compile> {
    //println!("{:?}", e);
    match e {
        Expr::Number(u) => {
            let u = *u;
                cont(Box::new(move |()| Value::Number(u)))
        },
        Expr::Add(l, r) => {
            let r = r.clone();
            let capture = compile(l, Rc::new(move |l_val| {
                if let Value::Number(l_num) = l_val(()) {
                    compile(&r, Rc::new(move |r_val| {
                        if let Value::Number(r_num) = r_val(()) {
                            let sum = l_num + r_num;
                            Box::new(move |()| Value::Number(sum))
                        } else {
                            panic!("can't add non-number right operand");
                        }
                    }))
                } else {
                    panic!("can't add non-number left operand");
                }
            }));
            cont(capture)
        },
        Expr::Suspend => {
            Box::new(move |()| { let cont = cont.clone(); Value::Coroutine(Rc::new(move |v| cont(v))) })
        },
        Expr::Resume(c, r) => {
            let r = r.clone();
            let capture = compile(c, Rc::new(move |coro_val| {
                if let Value::Coroutine(coro) = coro_val(()) {
                    compile(&r, Rc::new(move |r_val| {
                        coro(r_val)
                    }))
                } else {
                    panic!("can't resume non-coroutine");
                }
            }));
            cont(capture)
        }
    }
}


fn main() {
    let one = Expr::Add(
        Rc::new(Expr::Number(1)),
        Rc::new(Expr::Number(2)));
    let one_compiled = compile(&one,
        Rc::new(|v| v));
    println!("compiled one");
    let eval_one = one_compiled(());
    println!("{:?}", eval_one);
    //let two = compile(&Expr::Resume(
    //    Rc::new(Expr::Add(
    //        Rc::new(Expr::Suspend),
    //        Rc::new(Expr::Number(2)))),
    //    Rc::new(Expr::Number(1))),
    //        Rc::new(|v| v));
    //println!("compiled two");
    //let eval_two = two(());
    //println!("{:?}", eval_two);
    //let three = compile(&Expr::Resume(
    //    Rc::new(Expr::Add(
    //        Rc::new(Expr::Number(1)),
    //        Rc::new(Expr::Suspend))),
    //    Rc::new(Expr::Number(2))),
    //        Rc::new(|v| v));
    //println!("compiled three");
    //println!("{:?}", three(()));
}
