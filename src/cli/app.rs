use std::io;
use std::path::PathBuf;

use crate::fileio::todo_path;
use crate::todo_list::TodoList;
use clap::Parser;


/// A tree-like todo application that makes you smile
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Show a tree like output
    #[arg(short, long)]
    tree: bool,

    /// Show list of todos
    #[arg(short, long, default_value_t = true)]
    list: bool,

    /// Show done todos too
    #[arg(short='d', long)]
    show_done: bool,

    /// Show done todos too
    #[arg(short='s', long)]
    stdout: bool,
}
pub struct App {
    // todo_path: PathBuf,
    todo_list: TodoList,
    args: Args,
}


impl App {

    #[inline]
    pub fn new() -> Self {
        let todo_path = todo_path().unwrap();
        let args = Args::parse();
        let todo_list = TodoList::read(&todo_path, args.tree);
        App {
            args,
            // todo_path,
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
            Self::print_tree(&self.todo_list, self.args.show_done, 0)
        } else {
            self.print_list()
        }
        Ok(())
    }

    #[inline]
    pub fn print_tree(todo_list:&TodoList, show_done: bool, depth: i32) {
        let mut todos = todo_list.undone.todos.clone();
        if show_done {
            todos.extend(todo_list.done.todos.clone())
        }

        for (index, todo) in todos.iter().enumerate() {
            let is_last = index == todos.len() - 1;
            if depth > 0 {
                Self::print_indentation(depth, is_last);
            }
            println!("{}", todo.display(Some(show_done)));
            Self::print_tree(&todo.dependencies, show_done, depth+1);
        }
    }

    #[inline]
    fn print_indentation(depth: i32, is_last: bool) {
        for i in 0..depth {
            if i > 0 {
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
