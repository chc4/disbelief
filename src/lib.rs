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
trait Cont = Fn(Box<dyn Compile>) -> Box<dyn Compile>;

#[derive(Clone)]
pub enum Value {
    Atom(Atom),
    Quote(Vec<Expr>),
    Coroutine(Rc<dyn Fn(Value) -> Value>),
}

pub fn compile<'a>(e: &'a Expr, cont: Rc<dyn Cont>) -> Box<dyn Compile> {
    println!(".{:?}", e);
    match e {
        Expr::Constant(u) => {
            let u = u.clone();
            cont(Box::new(move |e| Value::Atom(u.clone())))
        },
        Expr::Application(f, args) => {
            let args = args.clone();
            //let arg_cont = Rc::new(move |e| e);
            compile(f, Rc::new(move |head| {

                use std::cell::RefCell;
                fn args_recurse(head: Rc<Box<dyn Compile>>, mut vals: Rc<RefCell<Vec<Box<dyn Compile>>>>, mut args: Rc<RefCell<Vec<Expr>>>) -> Box<dyn Compile> {
                    let next_val = args.borrow_mut().pop();
                    if let Some(next_val) = next_val {
                        compile(&next_val.clone(), Rc::new(move |next_val| {
                            vals.borrow_mut().push(next_val);
                            args_recurse(head.clone(), vals.clone(), args.clone())
                        }))
                    } else {
                        Box::new(move |e| {
                            let head = head(e);
                            let vals: Vec<_> = vals.borrow_mut().iter().map(|a| a(e)).collect();
                            println!("vals {:?}", vals);
                            match (head, vals.as_slice()) {
                                (Value::Atom(Atom::BuiltIn(BuiltIn::Plus)),
                                    [Value::Atom(Atom::Num(l)), Value::Atom(Atom::Num(r))])
                                => {
                                    let l = *l;
                                    let r = *r;
                                    Value::Atom(Atom::Num(l + r))
                                    //Box::new(move |()| panic!("{} {}", l, r))
                                },
                                _ => unimplemented!(),
                            }
                        })
                    }
                }

                let mut args_to_run = args.clone();
                args_to_run.reverse();
                args_recurse(Rc::new(head), Rc::new(RefCell::new(vec![])), Rc::new(RefCell::new(args_to_run)))

                //let first_arg = args.pop();
                //if let Some(first_arg) = first_arg {
                //    compile(first_arg, Rc::new(|first_arg| {
                //    }))
                //}
                //let arg_thunk: Rc<dyn Cont> = Rc::new(move |e| Vec::new(e));
                //for arg in args {
                //    arg_thunk = Rc::new(move |e| Vec::new(e).join(arg_thunk()) );
                //}
                //println!("application of {:?}", head);
                //match head {
                //    Value::Atom(Atom::BuiltIn(built)) => {
                //        let eval_args: Vec<_> = eval_args.drain(..).map(|a| a(()) ).collect();
                //        match (built, eval_args.as_slice()) {
                //            (BuiltIn::Plus, [Value::Atom(Atom::Num(l)), Value::Atom(Atom::Num(r))]) => {
                //                let l = *l;
                //                let r = *r;
                //                Box::new(move |()| Value::Atom(Atom::Num(l + r)))
                //                //Box::new(move |()| panic!("{} {}", l, r))
                //            },
                //            _ => unimplemented!(),
                //        }
                //    },
                //    x => unimplemented!("{:?}", x),
                //}
            }))
        },
        Expr::IfElse(cond, t, f) => {
            panic!()
        },
        Expr::Quote(q) => {
            let q = q.clone();
            cont(Box::new(move |e| Value::Quote(q.clone())))
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
            Box::new(move |e1| {
                let cont = cont.clone();
                println!("returning coroutine");
                Value::Coroutine(Rc::new(move |v| {
                    println!("running coroutine");
                    cont(Box::new(move |e| v.clone()))(e1)
                }))
            })
        },
        Expr::Resume(c, r) => {
            let r = compile(r, Rc::new(move |r_val| {
                println!("resume continuation");
                r_val
            }));
            let coro = compile(c, Rc::new(move |coro_val| {
                println!("coroutine continuation");
                coro_val
            }));
            cont(Box::new(move |e| {
                let coro_val = coro(e);
                let r_val = r(e);
                if let Value::Coroutine(coro) = coro_val {
                        println!("resuming coroutine");
                        coro(r_val)
                } else {
                    panic!("can't resume non-coroutine {:?}", coro_val);
                }
            }))
        }
    }
}



