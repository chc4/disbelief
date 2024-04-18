#![feature(trait_alias)]
pub mod tree;
pub mod parser;
use parser::{Atom, Expr, BuiltIn};
use std::rc::Rc;

use std::fmt::{Debug, Formatter};
impl Debug for Value {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Value::Atom(u) => write!(fmt, "{:?}", u),
            Value::Quote(q) => write!(fmt, "({})", q.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ")),
            Value::Coroutine(c) => write!(fmt, "<coroutine {:p}>", c),
        }
    }
}

//#[derive(Debug, Clone)]
//pub enum Expr {
//    Number(usize),
//    Add(Rc<Expr>, Rc<Expr>),
//    Suspend,
//    Resume(Rc<Expr>, Rc<Expr>),
//}

pub type Env = ();
trait Compile = Fn(Env) -> Value;
trait Cont = Fn(()) -> Box<dyn FnOnce(Rc<dyn Compile>) -> Rc<dyn Compile>>;

use std::cell::Cell;
#[derive(Clone)]
pub enum Value {
    Atom(Atom),
    Quote(Vec<Expr>),
    Coroutine(Rc<Cell<Box<dyn FnOnce(Value) -> Value>>>),
}

impl PartialEq for Value {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (Value::Atom(a), Value::Atom(b)) => a == b,
            (Value::Quote(a), Value::Quote(b)) => a == b,
            //(Value::Coroutine(a), Value::Coroutine(b)) => a.as_ptr() == b.as_ptr(),
            (Value::Coroutine(a), Value::Coroutine(b)) => true,
            _ => false
        }
    }
}

pub fn compile<'a>(e: &'a Expr, cont: Rc<dyn Cont>) -> Rc<dyn Compile> {
    println!(".{}", e);
    match e {
        Expr::Constant(u) => {
            let u = u.clone();
            let cont = cont(());
            cont(Rc::new(move |e| { println!("returning {}", u); Value::Atom(u.clone()) }))
        },
        Expr::Application(f, args) => {
            //let cont = cont(());
            let args = args.clone();
            compile(f, Rc::new(move |()| {
            let cont = cont.clone();
            let args = args.clone();
            Box::new(move |head| {

                use std::cell::RefCell;
                use std::collections::VecDeque;
                fn args_recurse(cont: Rc<dyn Cont>, head: Rc<dyn Compile>, mut vals: Rc<RefCell<Vec<Option<Rc<dyn Compile>>>>>, mut args: Rc<RefCell<VecDeque<Expr>>>) -> Rc<dyn Compile> {
                    let next_val = args.borrow_mut().pop_front();
                    if let Some(next_val) = next_val {
                        let next_val_e = next_val.clone();
                        let i = args.borrow().len();
                        vals.borrow_mut().push(None);
                        let v = args_recurse(cont, head, vals.clone(), args.clone());
                        compile(&next_val, Rc::new(move |()| {
                            let next_val_e = next_val_e.clone();
                            let v = v.clone();
                            let vals = vals.clone();
                            Box::new(move |next_val| {
                            println!("{} = {}", i, next_val_e);
                                vals.borrow_mut()[i] = Some(next_val);
                                //vals.borrow_mut().push(next_val);
                                v
                            }) }))
                    } else {
                        println!("running cont");
                        cont(())(Rc::new(move |e| {
                            let head = head(e);
                            let mut vals: Vec<_> = vals.borrow_mut().iter().rev().map(|a|
                                (a.clone().unwrap())(e)).collect();
                            println!("vals {:?}", vals);
                            match (head, vals.as_slice()) {
                                (Value::Atom(Atom::BuiltIn(BuiltIn::Plus)),
                                    [Value::Atom(Atom::Num(l)), Value::Atom(Atom::Num(r))])
                                => {
                                    Value::Atom(Atom::Num(l + r))
                                    //Box::new(move |()| panic!("{} {}", l, r))
                                },
                                _ => unimplemented!(),
                            }
                        }))
                    }
                }

                let mut args_to_run = args.clone();
                args_recurse(cont, head, Rc::new(RefCell::new(vec![])), Rc::new(RefCell::new(args_to_run.into())))
            })}))
        },
        Expr::IfElse(cond, t, f) => {
            panic!()
        },
        Expr::Quote(q) => {
            let q = q.clone();
            cont(())(Rc::new(move |e| Value::Quote(q.clone())))
        },
        //Expr::Add(l, r) => {
        //    let r = r.clone();
        //    let capture = compile(l, Rc::new(move |l_val| {
        //        if let Value::Number(l_num) = l_val(()) {
        //            compile(&r, Rc::new(move |r_val| {
        //                if let Value::Number(r_num) = r_val(()) {
        //                    let sum = l_num + r_num;
        //                    Box::new(move |()| Value::Number(sum))
        //                } else {
        //                    panic!("can't add non-number right operand");
        //                }
        //            }))
        //        } else {
        //            panic!("can't add non-number left operand");
        //        }
        //    }));
        //    cont(capture)
        //},
        Expr::Suspend => {
            let cont = cont.clone();
            Rc::new(move |e1| {
                println!("returning coroutine");
                let e1_once = Rc::new(Cell::new(e1));
                let cont = cont(());
                Value::Coroutine(Rc::new(Cell::new(Box::new(move |v: Value| {
                    println!("running coroutine");
                    cont(Rc::new(move |e| v.clone()))(e1_once.clone().take())
                }))))
            })
        },
        Expr::Resume(c, r) => {
            let coro_val = compile(c, Rc::new(|()| Box::new(move |coro_val| {
                println!("coroutine continuation");
                coro_val
            })));
            compile(r, Rc::new(move |()| {
                let coro_val = coro_val.clone();
                let cont = cont(());
                Box::new(move |r_val: Rc<dyn Compile>| {
                println!("resume continuation");
                cont(Rc::new(move |e| {
                    let coro_val = (coro_val.clone())(e);
                    let r_val = r_val(e);
                    if let Value::Coroutine(coro) = coro_val {
                            println!("resuming coroutine");
                            (coro.replace(Box::new(|val| panic!("attempted to resume already resumed coroutine"))))(r_val)
                    } else {
                        panic!("can't resume non-coroutine {:?}", coro_val);
                    }
                }))
            })}))
        }
    }
}



