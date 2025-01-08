use c3::todo_app::{App, TodoList};
use c3::TodoDisplay;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::{io::Write, process};
use std::{str, mem, thread};

pub fn fzf_search(app: &mut App) {
    let mut fzf = process::Command::new("fzf");
    fzf.stdin(process::Stdio::piped());
    fzf.stdout(process::Stdio::piped());
    let fzf_ps = fzf.spawn();
    if fzf_ps.is_err() {
        return;
    }
    // close the pipe
    let mut fzf_ps = fzf_ps.unwrap();
    let mut stdin = fzf_ps.stdin.take().unwrap();
    let mut output_selected = Arc::new(Mutex::new(false));
    let mut fzf_indices: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(vec![]));
    let fzf_indices_thread = Arc::clone(&mut fzf_indices);
    let output_thread = Arc::clone(&mut output_selected);
    let handle = thread::spawn(move ||{
        if let Ok(output) = fzf_ps.wait_with_output() {
            let mut output_thread = output_thread.lock().unwrap();
            *output_thread = true;
            if let Some(end_index) = output.stdout.iter().position(|ch| *ch == ' ' as u8) {
                if let Ok(indices_str) = str::from_utf8(&output.stdout[0..end_index]) {
                    *fzf_indices_thread.lock().unwrap() = indices_str.split(',').flat_map(|s| str::parse(s)).collect();
                }
            }
        }
    });
    let mut stack: Vec<(&TodoList, Vec<String>)> = vec![(app.current_list(), vec![])];
    while let Some((todolist, indices)) = stack.pop() {
        for (index, todo) in todolist.todos(app.get_restriction()).enumerate() {
            if *output_selected.lock().unwrap().deref() {
                break;
            }
            let current = todolist.true_position_in_list(index, app.get_restriction());
            stdin.write(format!("{},{current} {}\n",indices.join(","),todo.display_with_args(&app.args.display_args)).as_bytes());
            if let Some(list) = todo.dependency.as_ref().map(|dep| dep.todo_list()).flatten() {
                let mut new_indices = indices.clone();
                new_indices.push(current.to_string());

                stack.push((list, new_indices));
            }
        }
        if *output_selected.lock().unwrap().deref() {
            break;
        }
    }
    mem::drop(stdin);
    handle.join();
    app.index = fzf_indices.lock().unwrap().pop().unwrap();
    app.tree_path.append(fzf_indices.lock().unwrap().as_mut());
}
