use crate::todo_app::{App, TodoList};
use crate::TodoDisplay;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::{io, process};
use std::{str, mem, thread};

pub fn fzf_search(app: &mut App) {
    let mut fzf = process::Command::new("fzf");
    fzf.stdin(process::Stdio::piped());
    fzf.stdout(process::Stdio::piped());
    let fzf_ps = fzf.spawn();
    if fzf_ps.is_err() {
        return;
    }
    let mut fzf_ps = fzf_ps.unwrap();
    let mut stdin = fzf_ps.stdin.take().unwrap();
    let selected = Arc::new(Mutex::new(false));
    let fzf_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(vec![]));
    let fzf_indices_thread = Arc::clone(&fzf_indices);
    let selected_thread = Arc::clone(&selected);
    let handle = thread::spawn(move ||{
        if let Ok(output) = fzf_ps.wait_with_output() {
            *selected_thread.lock().unwrap() = true;
            if let Some(indices) = read_indices(&output.stdout) {
                *fzf_indices_thread.lock().unwrap() = indices;
            }
        }
    });
    write_todos(app, &mut stdin, selected);
    mem::drop(stdin);
    handle.join();
    let mut indices = fzf_indices.lock().unwrap();
    if let Some(item) = indices.pop() {
        app.index = item;
        app.tree_path.append(indices.as_mut());
    }
}

#[inline(always)]
pub fn read_indices(string: &[u8]) -> Option<Vec<usize>> {
    if let Some(end_index) = string.iter().position(|ch| *ch == b' ') {
        if let Ok(indices_str) = str::from_utf8(&string[0..end_index]) {
             return Some(indices_str.split(',').flat_map(str::parse).collect());
        }
    }
    None
}

#[inline(always)]
pub fn write_todos(app: &App, dst: &mut impl io::Write, selected: Arc<Mutex<bool>>) {
    let mut stack: Vec<(&TodoList, String)> = vec![(app.current_list(), String::new())];
    while let Some((todolist, indices)) = stack.pop() {
        for (index, todo) in todolist.todos(app.get_restriction()).enumerate() {
            if *selected.lock().unwrap().deref() {
                break;
            }
            let current = todolist.true_position_in_list(index, app.get_restriction());
            dst.write_all(format!("{indices},{current} {}\n",todo.display_with_args(&app.args.display_args)).as_bytes());
            if let Some(list) = todo.dependency.as_ref().and_then(|dep| dep.todo_list()) {
                let mut new_indices = indices.clone();
                new_indices.push_str(&current.to_string());
                new_indices.push(',');

                stack.push((list, new_indices));
            }
        }
        if *selected.lock().unwrap().deref() {
            break;
        }
    }
}
