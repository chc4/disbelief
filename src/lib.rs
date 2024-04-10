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
trait Compile = FnOnce(Env) -> Value;
trait Cont = FnOnce(()) -> Box<dyn FnOnce(Box<dyn Compile>) -> Box<dyn Compile>>;

use std::cell::Cell;
#[derive(Clone)]
pub enum Value {
    Atom(Atom),
    Quote(Vec<Expr>),
    Coroutine(Rc<Cell<Box<dyn FnOnce(Value) -> Value>>>),
}

pub fn compile<'a>(e: &'a Expr, cont: Box<dyn Cont>) -> Box<dyn Compile> {
    println!(".{}", e);
    match e {
        Expr::Constant(u) => {
            let u = u.clone();
            cont(())(Box::new(move |e| Value::Atom(u.clone())))
        },
        Expr::Application(f, args) => {
            let args = args.clone();
            compile(f, Box::new(move |()| {
            let args = args.clone();
            let cont = cont(());
            Box::new(move |head| {

                use std::cell::RefCell;
                fn args_recurse(cont: Box<dyn FnOnce(Box<dyn Compile>) -> Box<dyn Compile>>, head: Box<dyn Compile>, mut vals: Rc<RefCell<Vec<Cell<Option<Box<dyn Compile>>>>>>, mut args: Rc<RefCell<Vec<Expr>>>) -> Box<dyn Compile> {
                    let next_val = args.borrow_mut().pop();
                    if let Some(next_val) = next_val {
                        compile(&next_val.clone(), Box::new(move |()| {
                            let args = args.clone();
                            let vals = vals.clone();
                            use std::cell::RefCell;
                            let v = Rc::new(RefCell::new(Some(args_recurse(cont, head, vals.clone(), args.clone()))));
                            Box::new(move |next_val| {
                                vals.borrow_mut().push(Cell::new(Some(next_val)));
                                v.take().unwrap()
                            }) }))
                    } else {
                        println!("running cont");
                        cont(Box::new(move |e| {
                            let head = head(e);
                            let vals: Vec<_> = vals.borrow_mut().iter_mut().map(|a|
                                (a.replace(None).unwrap())(e)).collect();
                            println!("vals {:?}", vals);
                            match (head, vals.as_slice()) {
                                (Value::Atom(Atom::BuiltIn(BuiltIn::Plus)),
                                    [Value::Atom(Atom::Num(l)), Value::Atom(Atom::Num(r))])
                                => {
                                    let l = l;
                                    let r = r;
                                    Value::Atom(Atom::Num(l + r))
                                    //Box::new(move |()| panic!("{} {}", l, r))
                                },
                                _ => unimplemented!(),
                            }
                        }))
                    }
                }

                let mut args_to_run = args.clone();
                args_to_run.reverse();
                args_recurse(cont, head, Rc::new(RefCell::new(vec![])), Rc::new(RefCell::new(args_to_run)))
            })}))
        },
        Expr::IfElse(cond, t, f) => {
            panic!()
        },
        Expr::Quote(q) => {
            let q = q.clone();
            cont(())(Box::new(move |e| Value::Quote(q.clone())))
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
                let cont = cont(());
            Box::new(move |e1| {
                println!("returning coroutine");
                Value::Coroutine(Rc::new(Cell::new(Box::new(move |v: Value| {
                    println!("running coroutine");
                    cont(Box::new(move |e| v.clone()))(e1)
                }))))
            })
        },
        Expr::Resume(c, r) => {
            let coro_val = compile(c, Box::new(|()| Box::new(move |coro_val| {
                println!("coroutine continuation");
                coro_val
            })));
            use std::cell::RefCell;
            //let coro_val = Rc::new(RefCell::new(coro_val));
            let cont = cont(());
            compile(r, Box::new(move |()| {
                Box::new(move |r_val: Box<dyn Compile>| {
                println!("resume continuation");
                //let r_val = Rc::new(RefCell::new(r_val));
                cont(Box::new(move |e| {
                    let coro_val = coro_val(e);
                    let r_val = r_val(e);
                    if let Value::Coroutine(coro) = coro_val {
                            println!("resuming coroutine");
                            (coro.replace(Box::new(|val| panic!())))(r_val)
                    } else {
                        panic!("can't resume non-coroutine {:?}", coro_val);
                    }
                }))
            })}))
        }
    }
}



