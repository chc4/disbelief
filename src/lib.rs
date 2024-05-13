#![feature(trait_alias, let_chains)]
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
#[derive(Debug, Clone, Copy)]
pub enum Effect {
    Foo,
    Bar
}
trait Cont = Fn(Option<String>) -> Box<dyn Fn(Rc<dyn Compile>) -> Rc<dyn Compile>>;

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
            let cont = cont(None);
            cont(Rc::new(move |e| { println!("returning {}", u); Value::Atom(u.clone()) }))
        },
        Expr::Application(f, args) => {
            //let cont = cont(());
            let args = args.clone();
            compile(f, Rc::new(move |eff| {
            let cont = Cell::new(Some(cont.clone()));
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
                        compile(&next_val, Rc::new(move |eff| {
                            let next_val_e = next_val_e.clone();
                            let v = Cell::new(Some(v.clone()));
                            let vals = vals.clone();
                            Box::new(move |next_val| {
                            println!("{} = {}", i, next_val_e);
                                vals.borrow_mut()[i] = Some(next_val);
                                //vals.borrow_mut().push(next_val);
                                v.take().unwrap()
                            }) }))
                    } else {
                        println!("running cont");
                        cont(None)(Rc::new(move |e| {
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
                args_recurse(cont.take().unwrap(), head, Rc::new(RefCell::new(vec![])), Rc::new(RefCell::new(args_to_run.into())))
            })}))
        },
        Expr::IfElse(cond, t, f) => {
            panic!()
        },
        Expr::Quote(q) => {
            let q = q.clone();
            cont(None)(Rc::new(move |e| Value::Quote(q.clone())))
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
                // Fetch what our continuation will be
                let cont = cont(None);
                Value::Coroutine(Rc::new(Cell::new(Box::new(move |v: Value| {
                    println!("running coroutine");
                    // Return to where our continuation was once we're evaluated
                    cont(Rc::new(move |e| v.clone()))(e1_once.clone().take())
                }))))
            })
        },
        Expr::Resume(c, r) => {
            let coro_val = compile(c, Rc::new(|eff| Box::new(move |coro_val| {
                println!("coroutine continuation");
                coro_val
            })));
            let r_val = compile(r, Rc::new(move |eff| Box::new(move |r_val| {
                r_val
            })));
            let cont = cont(None);
            cont(Rc::new(move |e: Env| {
                let coro_val = (coro_val.clone())(e);
                if let Value::Coroutine(coro) = coro_val {
                    let r_val = r_val(e);
                    println!("resuming coroutine");
                    let coro_res = (coro.replace(Box::new(|val| panic!("attempted to resume already resumed coroutine"))))(r_val);
                    coro_res
                } else {
                    //panic!("can't resume non-coroutine {:?}", coro_val);
                    coro_val
                }
            }))
        },
        Expr::Handle(effect, res, coro) => {
            let effect = compile(effect, Rc::new(|eff| {
                 println!("handle_effect {:?}", eff);
                 Box::new(move |e_val| {
                     e_val
                 })
            }));
            let r_val = compile(res, Rc::new(move |eff| Box::new(move |r_val| {
                r_val
            })));

            // statically evaluate effect
            let can_handle = effect(());
            // default handler
            let default_cont = cont(None);
            if let Value::Atom(Atom::Keyword(can_handle)) = can_handle {
                let can_handle: String = can_handle.clone();
                let r_val2 = r_val.clone();
                let coro_val_ = compile(coro, Rc::new(move |eff| {
                    println!("handle_coro_effect {:?}", eff);
                    if let Some(ref eff) = eff && can_handle == *eff {
                        let cont = cont.clone();
                        let r_val = r_val.clone();
                        Box::new(move |coro_val| {
                            println!("handling coro effect");
                            let r_val = r_val.clone();
                            Rc::new(move |e: Env| {
                                let coro_val = coro_val(e);
                                println!("handling coro continuation, {:?}", coro_val);
                                //if let Value::Coroutine(coro) = coro_val {
                                    let r_val = (r_val.clone())(e);
                                    //(coro.replace(Box::new(|val| panic!("attempted to handle already resumed coroutine"))))(r_val)
                                    r_val
                                //} else {
                                //    coro_val
                                //}
                            })
                        })
                    } else {
                        println!("bubbling effect");
                        let eff = eff.clone();
                        let cont = cont.clone();
                        cont(eff)
                    }
                }));
                let r_val = r_val2.clone();
                Rc::new(move |e: Env| {
                    println!("running handle continuation");
                    let coro_val = coro_val_(e);
                    println!("coro_val {:?}", coro_val);
                    (coro_val)
                    //if let Value::Coroutine(coro) = coro_val {
                    //    println!("running handle coroutine continuation");
                    //    let r_val = r_val(e);
                    //    let coro_res = (coro.replace(Box::new(|val| panic!("attempted to handle already resumed coroutine"))))(r_val);
                    //    coro_res
                    //} else {
                    //    panic!()
                    //}
                })
                //compile(res, Rc::new(move |_eff| {
                //    let coro_val = compile(coro, Rc::new(move |eff| {
                //        let contc = contc.clone();
                //        println!("handle_coro_effect {:?}", eff);
                //        if let Some(ref eff) = eff && can_handle == *eff {
                //            println!("matching effect handle {}", can_handle);
                //            let contc = {contc.clone()};
                //            Box::new(move |coro_val| {
                //                let 
                //                let handle_val = handle_val.clone();
                //                panic!("handle coroutine continuation");
                //                contc(None)(Rc::new(move |e: Env| {
                //                    let coro_val = (coro_val.clone())(e);
                //                    let r_val = (handle_val.borrow_mut().unwrap())(e);
                //                    println!("handle res {:?}", coro_val);
                //                    if let Value::Coroutine(coro) = coro_val {
                //                        println!("handling coroutine");
                //                        (coro.replace(Box::new(|val| panic!("attempted to handle already resumed coroutine"))))(r_val)
                //                    } else {
                //                        coro_val
                //                    }
                //                }))
                //            })
                //        } else {
                //            println!("mismatched effect handle {} != {:?}", can_handle, eff);
                //            let contc = {contc.clone()};
                //            Box::new(move |coro_val| {
                //                panic!("forward coroutine continuation");
                //                println!("{:?}",coro_val(()));
                //                contc(eff.clone())(coro_val)
                //            })
                //        }
                //    }));
                //}))
            } else {
                panic!("can't handle non-keyword effect");
            }
            // handle(%foo, 1,
            // handle(%bar, 2,
            //   raise %foo
            // )
            // )
        },
        Expr::Raise(effect) => {
            let effect = compile(effect, Rc::new(|eff| {
                 println!("raise_effect {:?}", eff);
                 Box::new(move |coro_val| {
                     coro_val
                 })
            }));
            let effect = effect(());
            if let Value::Atom(Atom::Keyword(eff)) = effect {
                let cont = cont.clone();
                let cont = Rc::new(cont(Some(eff.clone())));
                Rc::new(move |e1| {
                    let cont = cont.clone();
                    println!("raising coroutine");
                    let e1_once = Rc::new(Cell::new(e1));
                    println!("raise :{}", eff.clone());
                    Value::Coroutine(Rc::new(Cell::new(Box::new(move |v: Value| {
                        println!("running coroutine");
                        (cont)(Rc::new(move |e| v.clone()))(e1_once.clone().take())
                    }))))
                })

            } else {
                panic!("can't raise non-keyword effect");
            }
        },
    }
}



