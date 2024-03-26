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
    c.bench_function("closure-compiler direct", |b| {
        use hysteresis::*;
        let one = Expr::Add(
            Rc::new(Expr::Number(1)),
            Rc::new(Expr::Number(2)));
        let one = hysteresis::compile(&one,
            Rc::new(|v| v));
        b.iter(|| one(black_box(())))
    });
    c.bench_function("closure-compiler left suspect", |b| {
        use hysteresis::*;
        let two = Expr::Resume(
            Rc::new(Expr::Add(
                Rc::new(Expr::Suspend),
                Rc::new(Expr::Number(2)))),
            Rc::new(Expr::Number(1)));
        let two = hysteresis::compile(&two,
                Rc::new(|v| v));
        //println!("compiled two");
        b.iter(|| two(black_box(())))
    });
    c.bench_function("closure-compiler right suspend", |b| {
        use hysteresis::*;
        let three = Expr::Resume(
        Rc::new(Expr::Add(
            Rc::new(Expr::Number(1)),
            Rc::new(Expr::Suspend))),
        Rc::new(Expr::Number(2)));
        let three = hysteresis::compile(&three,
                Rc::new(|v| v));
        //println!("compiled three");
        b.iter(|| three(black_box(())))
    });

    c.bench_function("tree-walk direct", |b| {
        use hysteresis::tree::*;
        let one = Expr::Add(
            Box::new(Expr::Number(1)),
            Box::new(Expr::Number(2)));
        b.iter(|| eval(&one, Box::new(|v| v)));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
