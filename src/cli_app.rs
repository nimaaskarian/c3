use std::io;
use super::todo_app::{App ,TodoList};

#[inline]
pub fn run(app: &mut App) -> io::Result<()>{
    let app = CliApp::new(app);
    app.print()?;
    Ok(())
}

pub struct CliApp<'a> {
    todo_app: &'a mut App,
}

impl <'a>CliApp <'a>{
    #[inline]
    pub fn new(app: &'a mut App) -> Self {
        CliApp {
            todo_app: app,
        }
    }

    #[inline]
    fn print_list(&self) {
        for display in self.todo_app.display() {
            println!("{}", display);
        }
    }

    #[inline]
    pub fn print(&self) -> io::Result<()>{
        if self.todo_app.args.stdout {
            self.todo_app.print()?;
            return Ok(())
        }
        if self.todo_app.is_tree() {
            Self::print_tree(&self.todo_app.todo_list, self.todo_app.args.show_done, 0, vec![false])
        } else {
            self.print_list()
        }
        Ok(())
    }

    #[inline]
    pub fn print_tree(todo_list:&TodoList, show_done: bool, depth: usize, was_last: Vec<bool>) {
        let mut todos = todo_list.undone.todos.clone();
        if show_done {
            todos.extend(todo_list.done.todos.clone())
        }

        for (index, todo) in todos.iter().enumerate() {
            let is_last = index == todos.len() - 1;
            if depth > 0 {
                Self::print_indentation(depth, is_last, &was_last);
            }
            println!("{}", todo.display(Some(show_done)));
            let mut was_last_clone = was_last.clone();
            was_last_clone.push(is_last);
            if let Some(todo_list) = todo.dependency.todo_list() {
                Self::print_tree(&todo_list, show_done, depth+1, was_last_clone);
            }
        }
    }

    #[inline]
    fn print_indentation(depth: usize, is_last: bool, was_last: &Vec<bool>) {
        for i in 1..depth {
            if was_last[i+1] {
                print!("    ")
            } else {
                print!("│   ")
            }
        }
        if is_last {
            print!("└── ");
        } else {
            print!("├── ");
        }
    }
}
