use super::todo_app::{App, RestrictionFunction, Todo, TodoList};
use crate::DisplayArgs;
use std::io;

#[inline]
pub fn run(app: &mut App) -> io::Result<()> {
    let app = CliApp::new(app);
    app.print()?;
    Ok(())
}

pub struct CliApp<'a> {
    todo_app: &'a App,
}

impl<'a> CliApp<'a> {
    #[inline]
    pub fn new(app: &'a mut App) -> Self {
        for message in app.args.append_todo.clone() {
            app.append(message);
        }

        if let Some(path) = app.args.output_file.clone() {
            app.output_list_to_path(&path);
        }

        for message in app.args.prepend_todo.clone() {
            app.prepend(message);
        }
        if let Some(path) = app.args.append_file.clone() {
            app.append_list_from_path(path)
        }
        app.do_commands_on_selected();
        let _ = app.write();
        CliApp { todo_app: app }
    }

    #[inline]
    fn print_list(&self) {
        for display in self.todo_app.display_current() {
            println!("{}", display);
        }
    }

    #[inline]
    pub fn print(&self) -> io::Result<()> {
        if !self.todo_app.args.search_and_select.is_empty() {
            self.todo_app.print_selected();
            return Ok(());
        }
        if self.todo_app.args.stdout {
            self.todo_app.print()?;
            return Ok(());
        }
        if self.todo_app.is_tree() {
            let mut print_todo = PrintTodoTree::new(self.todo_app.args.minimal_tree);
            print_todo.print_list(
                &self.todo_app.todo_list,
                &self.todo_app.args.display_args,
                self.todo_app.restriction(),
            );
        } else {
            self.print_list()
        }
        Ok(())
    }
}

// TODO: Use traverse_tree instead of this struct for printing todo tree.
#[derive(Clone)]
struct PrintTodoTree {
    last_stack: Vec<bool>,
    should_print_indention: bool,
    is_last: bool,
}

impl PrintTodoTree {
    #[inline]
    pub fn new(should_print_indention: bool) -> Self {
        PrintTodoTree {
            last_stack: vec![],
            is_last: false,
            should_print_indention,
        }
    }

    #[inline]
    pub fn tree_child(&self) -> Self {
        let mut child = self.clone();
        child.last_stack.push(self.what_to_push());

        child
    }

    #[inline]
    pub fn what_to_push(&self) -> bool {
        let popped = self.last_stack.last();
        !self.is_last && popped.is_some() && !self.last_stack.is_empty()
    }

    #[inline]
    pub fn print_list(
        &mut self,
        todo_list: &TodoList,
        display_args: &DisplayArgs,
        restriction: &RestrictionFunction,
    ) {
        let todos = todo_list.todos(restriction);

        for (index, todo) in todos.iter().enumerate() {
            self.is_last = index == todos.len() - 1;
            if !self.last_stack.is_empty() {
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
        println!("{}", todo.display(display_args));
    }

    #[inline]
    fn print_note(&mut self, note: &str) {
        let mut last_stack = self.last_stack.clone();
        last_stack.push(self.what_to_push());

        for line in note.lines() {
            self.print_prenote(last_stack.clone());
            println!("{}", line);
        }
    }

    #[inline]
    fn print_prenote(&self, last_stack: Vec<bool>) {
        self.print_preindention(last_stack);
        print!("    ")
    }

    #[inline]
    fn print_indention(&self) {
        if self.should_print_indention {
            return;
        }
        self.print_preindention(self.last_stack.clone());
        if self.is_last {
            print!("└── ");
        } else {
            print!("├── ");
        }
    }

    #[inline(always)]
    fn print_preindention(&self, last_stack: Vec<bool>) {
        let mut stack_iter = last_stack.into_iter();
        stack_iter.next();
        for x in stack_iter {
            if x {
                print!("│   ")
            } else {
                print!("    ")
            }
        }
    }
}
