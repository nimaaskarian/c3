use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::{Arc, Mutex};
use std::{env, fs::File, hint::black_box, path::PathBuf};

use c3::{todo_app::App, AppArgs};
use c3::todo_app::fzf_search;

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
    c.bench_function("reorder 500k todos", |b| {
        b.iter(|| {
            app.index = 400000;
            black_box(&mut app).set_current_priority(1);
        })
    });
}

fn display(c: &mut Criterion) {
    let mut app = App::new(AppArgs {
        todo_path: PathBuf::from("../fuckc3-todo"),
        ..Default::default()
    });
    c.bench_function("display 500k todos", |b| {
        b.iter(|| {
            black_box(&mut app).display_current_list();
        })
    });
}

fn write_to_stdout(c: &mut Criterion) {
    let app = App::new(AppArgs {
        todo_path: PathBuf::from("../fuckc3-todo"),
        ..Default::default()
    });
    c.bench_function("write to stdout 500k todos", |b| {
        b.iter(|| black_box(&app.todo_list).write_to_stdout())
    });
}

fn batch_edit(c: &mut Criterion) {
    let mut app = App::new(AppArgs {
        todo_path: PathBuf::from("../fuckc3-todo"),
        ..Default::default()
    });
    env::set_var("EDITOR", "cat");
    c.bench_function("batch edit 500k todos", |b| {
        b.iter(|| black_box(&mut app).batch_editor_messages())
    });
}

fn write_to_fzf(c: &mut Criterion) {
    let mut app = App::new(AppArgs {
        todo_path: PathBuf::from("../fuckc3-10000-todo"),
        ..Default::default()
    });
    let mut file = File::open("/dev/null").unwrap();
    let selected = Arc::new(Mutex::new(false));
    c.bench_function("write 10k todos to fzf", |b| {
        b.iter(|| {
            let selected = Arc::clone(&selected);
            fzf_search::write_todos(black_box(&mut app), &mut file, selected)
        })
    });
}

criterion_group!(benches,write_to_fzf, sort, reorder, display, write_to_stdout, batch_edit);
criterion_main!(benches);
