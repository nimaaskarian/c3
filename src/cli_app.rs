use clap::{Command, CommandFactory};
use super::todo_app::{App, RestrictionFunction, Todo, TodoList};
use crate::{CliArgs, DisplayArgs, DoOnSelected};
use crate::Args;
use clap_complete::{generate, Generator};
use std::io;
use std::process;

pub struct NotCli;
#[inline]
pub fn run(app: &mut App, args: CliArgs) -> Result<(),NotCli> {
    if !args.search_and_select.is_empty() {
        for query in args.search_and_select {
            app.set_query_restriction(query, None)
        }
        if app.is_todos_empty() {
            process::exit(1);
        }
        let restriction = app.restriction().clone();
        if let Some(do_on_selected) = args.do_on_selected {
            match do_on_selected {
                DoOnSelected::Delete => {
                    app.current_list_mut().todos.retain(|todo| !restriction(todo))
                }
                DoOnSelected::Done => {
                    for todo in app.current_list_mut().todos_mut(&restriction) {
                        todo.set_done(true);
                    }
                }
            }
        } else {
            print_todos(app);
            return Ok(())
        }
    }
    if args.batch_edit {
        app.batch_editor_messages();
    }
    if app.is_changed() {
        app.write();
    }
    if args.print_path {
        println!("{}", app.args.todo_path.to_str().unwrap_or(""));
        let notes = app.args.todo_path.parent().unwrap().join("notes");
        if notes.is_dir() {
            println!("{}", notes.to_str().unwrap_or(""));
        }
        return Ok(())
    }
    if let Some(generator) = args.completion {
        print_completions(generator, &mut Args::command());
        return Ok(())
    }

    if args.stdout {
        app.write_to_stdout();
        return Ok(())
    }
    if args.minimal_tree || args.list {
        if app.args.no_tree {
            print_todos(app);
        } else {
            let mut print_todo = PrintTodoTree::new(args.minimal_tree);
            print_todo.print_list(
                &app.todo_list,
                &app.args.display_args,
                app.restriction(),
            )
        }
        return Ok(())
    }
    if let Some(path) = args.output_file.as_ref() {
        app.output_list_to_path(path);
        return Ok(())
    }
    Err(NotCli)
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
