use clap::{Command, CommandFactory};
use super::todo_app::{App, RestrictionFunction, Todo, TodoList};
use crate::{CliMode, DisplayArgs};
use crate::Args;
use clap_complete::{generate, Generator};
use std::io;

#[inline]
pub fn run(app: &mut App, mode: CliMode) -> io::Result<()> {
    match mode {
        CliMode::Stdout => {
            app.write_to_stdout()?;
        }
        CliMode::Completion(generator) => {
            print_completions(generator, &mut Args::command());
        }
        CliMode::PrintTree(is_minimal) => {
            let mut print_todo = PrintTodoTree::new(is_minimal);
            print_todo.print_list(
                &app.todo_list,
                &app.args.display_args,
                app.restriction(),
            )
        }
        CliMode::Print => print_todos(app),
    }
    Ok(())
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn print_todos(app: &App) {
    for display in app.display_current() {
        println!("{}", display);
    }
}

// TODO: Use traverse_tree instead of this struct for printing todo tree.
#[derive(Clone)]
struct PrintTodoTree {
    last_stack: Vec<bool>,
    should_skip_indention: bool,
    is_last: bool,
}

impl PrintTodoTree {
    #[inline]
    pub fn new(should_skip_indention: bool) -> Self {
        PrintTodoTree {
            last_stack: vec![],
            is_last: false,
            should_skip_indention,
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

            if let Some(todo_list) = todo.dependency.as_ref().and_then(|dep| dep.todo_list()) {
                let mut tree_child = self.tree_child();
                tree_child.print_list(todo_list, display_args, restriction);
            } else if let Some(note) = todo.dependency.as_ref().and_then(|dep| dep.note()) {
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
        for line in note.lines() {
            let last = if self.last_stack.is_empty() {
                None
            } else {
                Some(self.what_to_push())
            };
            self.print_prenote(&self.last_stack, last);
            println!("{}", line);
        }
    }

    #[inline]
    fn print_prenote(&self, last_stack: &[bool], last_item: Option<bool>) {
        if !self.should_skip_indention {
            self.print_preindention(last_stack, last_item);
        }
        print!("   ")
    }

    #[inline]
    fn print_indention(&self) {
        if self.should_skip_indention {
            return;
        }
        self.print_preindention(&self.last_stack, None);
        if self.is_last {
            print!("└── ");
        } else {
            print!("├── ");
        }
    }

    #[inline(always)]
    fn print_preindention(&self, last_stack: &[bool], last_item: Option<bool>) {
        let mut stack_iter = last_stack.iter();
        stack_iter.next();
        for &x in stack_iter.chain(last_item.as_ref()) {
            if x {
                print!("│   ")
            } else {
                print!("    ")
            }
        }
    }
}
