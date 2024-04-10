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
trait Cont = Fn(()) -> Box<dyn Fn(Box<dyn Compile>) -> Box<dyn Compile>>;

#[derive(Clone)]
pub enum Value {
    Atom(Atom),
    Quote(Vec<Expr>),
    Coroutine(Rc<dyn Fn(Value) -> Value>),
}

pub fn compile<'a>(e: &'a Expr, cont: Rc<dyn Cont>) -> Box<dyn Compile> {
    println!(".{}", e);
    match e {
        Expr::Constant(u) => {
            let u = u.clone();
            cont(())(Box::new(move |e| Value::Atom(u.clone())))
        },
        Expr::Application(f, args) => {
            //I think this is still not quite right: we only call compile in the
            //continuation of each argument, which means that we don't fully execute
            //closure compilation if there is a suspend until we resume it
            //I tried to do compilation all at once and then only execute the argument
            //closures after, but I couldn't get the continuations working correctly.
            //I think this is a symptom of the fact our compile continuations need
            //the dyn Compile objects, and for suspends we don't have anything until
            //we resume, so we can't fully resolve the delimited continuation for nested
            //suspends. I think we'd need to introduce another closure that holds the
            //continuations, like we are doing for expressions, so that Suspend
            //could do `cont()` outside the Compile, collapsing the argument thunks
            let cont = cont.clone();
            let args = args.clone();
            compile(f, Rc::new(move |()| {
            let args = args.clone();
            let cont = cont.clone();
            let cont = Rc::new(cont(()));
            Box::new(move |head| {

                use std::cell::RefCell;
                fn args_recurse(cont: Rc<Box<dyn Fn(Box<dyn Compile>) -> Box<dyn Compile>>>, head: Rc<Box<dyn Compile>>, mut vals: Rc<RefCell<Vec<Box<dyn Compile>>>>, mut args: Rc<RefCell<Vec<Expr>>>) -> Box<dyn Compile> {
                    let next_val = args.borrow_mut().pop();
                    if let Some(next_val) = next_val {
                        compile(&next_val.clone(), Rc::new(move |()| {
                            let args = args.clone();
                            let head = head.clone();
                            let vals = vals.clone();
                            let cont = cont.clone();
                            use std::cell::RefCell;
                            let v = Rc::new(RefCell::new(Some(args_recurse(cont.clone(), head.clone(), vals.clone(), args.clone()))));
                            Box::new(move |next_val| {
                                vals.borrow_mut().push(next_val);
                                v.clone().take().unwrap()
                            }) }))
                    } else {
                        println!("running cont");
                        cont(Box::new(move |e| {
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
                        }))
                    }
                }

                let mut args_to_run = args.clone();
                args_to_run.reverse();
                args_recurse(cont.clone(), Rc::new(head), Rc::new(RefCell::new(vec![])), Rc::new(RefCell::new(args_to_run)))
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
                let cont = cont.clone();
                let cont = Rc::new(cont(()));
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
            let coro_val = compile(c, Rc::new(|()| Box::new(move |coro_val| {
                println!("coroutine continuation");
                coro_val
            })));
            use std::cell::RefCell;
            let coro_val = Rc::new(RefCell::new(coro_val));
            let cont = cont.clone();
            let cont = Rc::new(cont(()));
            compile(r, Rc::new(move |()| {
                let coro_val = coro_val.clone();
                let cont = cont.clone();
                Box::new(move |r_val| {
                println!("resume continuation");
                let r_val = Rc::new(RefCell::new(r_val));
                let coro_val = coro_val.clone();
                cont(Box::new(move |e| {
                    let coro_val = (coro_val.borrow_mut())(e);
                    let r_val = (r_val.borrow_mut())(e);
                    if let Value::Coroutine(coro) = coro_val {
                            println!("resuming coroutine");
                            coro(r_val)
                    } else {
                        panic!("can't resume non-coroutine {:?}", coro_val);
                    }
                }))
            })}))
        }
    }
}



