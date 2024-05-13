use std::rc::Rc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};


fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    use disbelief::*;
    use disbelief::parser::*;
    use disbelief::tree::eval;
    use disbelief::Compile;
    fn compile_one<T: Into<String>>(s: T) -> Rc<dyn Compile> {
        let e = parse_expr(&s.into()).unwrap().1;
        let compiled = compile(&e, Rc::new(|eff| Box::new(|v| v)));
        compiled
    }

    c.bench_function("closure-compiler direct", |b| {
        let one = compile_one("(+ 1 2)");
        b.iter(|| one(black_box(())))
    });
    c.bench_function("closure-compiler left suspend", |b| {
        use disbelief::*;
        let two = compile_one("(+ suspend 2)");
        //println!("compiled two");
        b.iter(|| two(black_box(())))
    });
    c.bench_function("closure-compiler right suspend", |b| {
        use disbelief::*;
        let three = compile_one("(+ 1 suspend)");
        //println!("compiled three");
        b.iter(|| three(black_box(())))
    });

    c.bench_function("tree-walk direct", |b| {
        use disbelief::tree::*;
        let one = Expr::Add(
            Box::new(Expr::Number(1)),
            Box::new(Expr::Number(2)));
        b.iter(|| eval(&one, Box::new(|v| v)));
    });

    c.bench_function("tree-walk left suspend", |b| {
        use disbelief::tree::*;
        let two = Expr::Resume(Box::new(Expr::Add(
            Box::new(Expr::Suspend),
            Box::new(Expr::Number(2)))), Box::new(Expr::Number(1)));
        b.iter(|| eval(&two, Box::new(|v| v)));
    });

    c.bench_function("tree-walk right suspend", |b| {
        use disbelief::tree::*;
        let one = Expr::Resume(Box::new(Expr::Add(
            Box::new(Expr::Number(1)),
            Box::new(Expr::Suspend))), Box::new(Expr::Number(2)));
        b.iter(|| eval(&one, Box::new(|v| v)));
    });

}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
