use std::{hint::black_box, path::PathBuf};
use criterion::{criterion_group, criterion_main, Criterion};

use c3::{todo_app::App, AppArgs};

fn sort(c: &mut Criterion) {
    let mut app = App::new(AppArgs {
        todo_path: PathBuf::from("../fuckc3-todo"),
        ..Default::default()
    });
    c.bench_function("sort 500k todos", |b| b.iter(|| black_box(&mut app).read()));
}

fn reorder(c: &mut Criterion) {
    let mut app = App::new(AppArgs {
        todo_path: PathBuf::from("../fuckc3-todo"),
        ..Default::default()
    });
    c.bench_function("reorder 500k todos", |b| b.iter(|| {
        app.index = 400000;
        black_box(&mut app).set_current_priority(1);
    }));
}

fn display(c: &mut Criterion) {
    let mut app = App::new(AppArgs {
        todo_path: PathBuf::from("../fuckc3-todo"),
        ..Default::default()
    });
    c.bench_function("display 500k todos", |b| b.iter(|| {
        black_box(&mut app).display_current();
    }));
}

criterion_group!(benches, sort, reorder, display);
criterion_main!(benches);
