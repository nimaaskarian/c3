use std::io;

use crate::fileio::todo_path;
use crate::todo_list::TodoList;
use crate::Args;

pub struct App {
    todo_list: TodoList,
    args: Args,
}

impl App {

    #[inline]
    pub fn new(args: Args) -> Self {
        let todo_list = match &args.todo_path {
            Some(value) => TodoList::read(value, args.tree, true),
            None => {
                let todo_path = todo_path().unwrap();
                TodoList::read(&todo_path, args.tree, true)
            }
        };
        App {
            args,
            todo_list,
        }
    }

    #[inline]
    fn print_list(&self) {
        for todo_str in self.todo_list.display(self.args.show_done) {
            println!("{}", todo_str);
        }
    }

    #[inline]
    pub fn print(&self) -> io::Result<()>{
        if self.args.stdout {
            self.todo_list.print()?;
            return Ok(())
        }
        if self.args.tree {
            Self::print_tree(&self.todo_list, self.args.show_done, 0, vec![false])
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
