use hysteresis::parser::*;
use hysteresis::*;

use std::rc::Rc;

fn main() {
    let id = Rc::new(|v| v);
    let tests = [
        "1",
        "(+ 1 2)",
        "(+ (+ 1 2) 3)",
        "(+ 1 (+ 2 3))",
        "suspend",
        "resume suspend 1",
        "resume (+ suspend 2) 1",
        "resume resume (+ suspend suspend) 1 2",
        //"(+ suspend 2)",
        //"resume (+ suspend 2) 1",
    ];
    for test in tests {
        println!("---");
        let e = parse_expr(test).unwrap().1;
        let one_compiled = compile(&e, id.clone());
        println!("=> compiled one");
        let eval_one = one_compiled(());
        println!("{:?}", eval_one);
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
