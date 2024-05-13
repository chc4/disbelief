use disbelief::parser::*;
use disbelief::*;

use std::rc::Rc;
use core::num::Wrapping;

fn main() {
    //let id = Rc::new(|()| Box::new(|v| v));
    fn one_shot<T: Into<String>>(s: T) -> Value {
        let e = parse_expr(&s.into()).unwrap().1;
        let compiled = compile(&e, Rc::new(|eff| Box::new(|v| v)));
        compiled(())
    }
    let tests = [
        ("1", None),
        ("(+ 1 2)", None),
        ("(+ (+ 1 2) 3)", None),
        ("(+ 1 (+ 2 3))", None),
        ("suspend", None),
        ("resume suspend 1", Some(one_shot("1"))),
        ("resume (+ suspend 2) 1", Some(one_shot("3"))),
        ("resume (+ 1 suspend) 2", Some(one_shot("3"))),
        ("resume resume (+ suspend suspend) 1 2", Some(one_shot("3"))),
        ("resume resume suspend suspend (+ 1 2)", Some(one_shot("3"))),
        ("resume resume (+ (+ 1 suspend) suspend) 2 3", Some(one_shot("6"))),
        ("resume 1 suspend", Some(one_shot("1"))),
        (":a", None),
        //("handle :foo 1 2", Some(one_shot("2"))),
        //("handle :foo 1 suspend", Some(one_shot("suspend"))),
        ("resume handle :foo 1 raise :bar 2", Some(one_shot("2"))),
        ("resume handle :foo 1 raise :foo 2", Some(one_shot("1"))),
        ("resume handle :foo 1 handle :bar 2 raise :bar 3", Some(one_shot("2"))),
        ("resume handle :foo 1 handle :bar 2 raise :foo 3", Some(one_shot("1"))),
        //"(+ suspend 2)",
        //"resume (+ suspend 2) 1",
    ];
    for (test, gold) in tests {
        println!("---");
        let e = parse_expr(test).unwrap().1;
        let one_compiled = compile(&e, Rc::new(|eff| Box::new(|v| v)));
        println!("=> compiled one");
        let eval_one = one_compiled(());
        println!("+{:?}", eval_one);
        let eval_two = one_compiled(());
        println!("-{:?}", eval_two);
        assert_eq!(eval_one, eval_two);
        if let Some(gold) = gold {
            assert_eq!(eval_one, gold, "Failed golden test: {:?}", gold);
        }
    }
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
