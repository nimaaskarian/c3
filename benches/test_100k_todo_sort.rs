use std::{hint::black_box, path::PathBuf};
use criterion::{criterion_group, criterion_main, Criterion};

use c3::{todo_app::App, AppArgs};

fn criterion_benchmark(c: &mut Criterion) {
    let mut app = App::new(AppArgs {
        todo_path: PathBuf::from("../fuckc3-100000-todo"),
        ..Default::default()
    });
    c.bench_function("sort 100k todo", |b| b.iter(|| black_box(&mut app).read()));
}
criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
