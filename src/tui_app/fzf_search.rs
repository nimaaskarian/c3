use c3::todo_app::{App, TodoList};
use c3::TodoDisplay;
use std::{io::Write, process};
use std::{str, mem};

pub fn fzf_search(app: &mut App) -> Vec<usize> {
    let mut fzf = process::Command::new("fzf");
    fzf.stdin(process::Stdio::piped());
    fzf.stdout(process::Stdio::piped());
    let fzf_ps = fzf.spawn();
    if fzf_ps.is_err() {
        return vec![];
    }
    let mut fzf_ps = fzf_ps.unwrap();
    let mut stdin = fzf_ps.stdin.take().unwrap();
    let mut stack: Vec<(&TodoList, Vec<String>)> = vec![(app.current_list(), vec![])];
    while let Some((todolist, indices)) = stack.pop() {
        for (index, todo) in todolist.todos(app.get_restriction()).enumerate() {
            let current = todolist.true_position_in_list(index, app.get_restriction());
            stdin.write(format!("{},{current} {}\n",indices.join(","),todo.display_with_args(&app.args.display_args)).as_bytes());
            if let Some(list) = todo.dependency.as_ref().map(|dep| dep.todo_list()).flatten() {
                let mut new_indices = indices.clone();
                new_indices.push(current.to_string());

                stack.push((list, new_indices));
            }
        }
    }
    // close the pipe
    mem::drop(stdin);
    if let Ok(output) = fzf_ps.wait_with_output() {
        if let Some(end_index) = output.stdout.iter().position(|ch| *ch == ' ' as u8) {
            if let Ok(indices_str) = str::from_utf8(&output.stdout[0..end_index]) {
                let mut fzf_indices: Vec<usize> = indices_str.split(',').flat_map(|s| str::parse(s)).collect();
                app.index = fzf_indices.pop().unwrap();
                app.tree_path.append(&mut fzf_indices);
            }
        }
    }
    vec![]
}
