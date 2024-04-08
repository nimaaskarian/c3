use std::io;
use super::todo_app::{App, TodoList, Todo, RestrictionFunction};
use crate::DisplayArgs;

#[inline]
pub fn run(app: &mut App) -> io::Result<()>{
    let app = CliApp::new(app);
    app.print()?;
    Ok(())
}

pub struct CliApp<'a> {
    todo_app: &'a App,
}

impl <'a>CliApp <'a>{
    #[inline]
    pub fn new(app: &'a mut App) -> Self {
        for message in app.args.append_todo.clone() {
            app.append(message);
        }
        for message in app.args.prepend_todo.clone() {
            app.prepend(message);
        }
        if let Some(path) = app.args.append_file.clone() {
            app.append_list_from_path(path)
        }
        app.do_commands_on_selected();
        if !app.args.append_todo.is_empty() || !app.args.prepend_todo.is_empty() || app.is_changed(){
            let _ = app.write();
        }
        CliApp {
            todo_app: app,
        }
    }

    #[inline]
    fn print_list(&self) {
        for display in self.todo_app.display_current() {
            println!("{}", display);
        }
    }

    #[inline]
    pub fn print(&self) -> io::Result<()>{
        if !self.todo_app.args.search_and_select.is_empty() {
            self.todo_app.print_selected();
            return Ok(())
        }
        if self.todo_app.args.stdout {
            self.todo_app.print()?;
            return Ok(())
        }
        if self.todo_app.is_tree() {
            let mut print_todo = PrintTodoTree::new(self.todo_app.args.minimal_tree);
            print_todo.print_list(&self.todo_app.todo_list, &self.todo_app.args.display_args, &self.todo_app.restriction);
        } else {
            self.print_list()
        }
        Ok(())
    }
}

// TODO: Use traverse_tree instead of this struct for printing todo tree. 
#[derive(Clone)]
struct PrintTodoTree {
    was_last: Vec<bool>,
    should_print_indention: bool,
    is_last: bool,
    depth: usize,
}

impl PrintTodoTree {
    #[inline]
    pub fn new(should_print_indention:bool) -> Self {
        PrintTodoTree {
            was_last: vec![],
            is_last: false,
            depth: 0,
            should_print_indention,
        }
    }

    #[inline]
    pub fn tree_child(&self) -> Self {
        let mut new_print = self.clone();
        new_print.depth+=1;
        new_print.was_last.push(self.is_last);

        new_print
    }

    #[inline]
    pub fn print_list(&mut self, todo_list: &TodoList, display_args: &DisplayArgs, restriction: &RestrictionFunction) {
        let todos = todo_list.todos(restriction);

        for (index, todo) in todos.iter().enumerate() {
            self.is_last = index == todos.len() - 1;
            if self.depth > 0 {
                self.print_indention();
            }
            self.print_todo(todo, display_args);

            if let Some(todo_list) = todo.dependency.todo_list() {
                let mut tree_child = self.tree_child();
                tree_child.print_list(todo_list, display_args, restriction);
            }

            if let Some(note) = todo.dependency.note() {
                self.print_note(note)
            }
        }
    }

    #[inline]
    fn print_todo(&self, todo: &Todo, display_args: &DisplayArgs) {
        println!("{}", todo.display(&display_args));
    }

    #[inline]
    fn print_note(&mut self, note: &String) {
        self.was_last.push(self.is_last);
        self.is_last = true;

        let mut lines = note.lines();
        self.print_indention_with_depth(self.depth+1);
        if let Some(line) = lines.next() {
            println!("{}", line);
        }
        for line in lines {
            self.print_prenote();
            println!("{}", line);
        }
    }

    #[inline]
    fn print_prenote(&self) {
        for i in 0..self.depth {
            if self.was_last[i+1] {
                print!("    ")
            } else {
                print!("│   ")
            }
        }
        print!("    ")
    }

    #[inline]
    fn print_indention_with_depth(&self, depth: usize) {
        if self.should_print_indention {
            return
        }
        for i in 1..depth {
            if self.was_last[i] {
                print!("    ")
            } else {
                print!("│   ")
            }
        }
        if self.is_last {
            print!("└── ");
        } else {
            print!("├── ");
        }
    }

    #[inline]
    fn print_indention(&self) {
        self.print_indention_with_depth(self.depth);
    }
}
