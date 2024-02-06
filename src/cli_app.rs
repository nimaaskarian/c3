use std::io;
use super::todo_app::{App ,TodoList, Todo};

#[inline]
pub fn run(app: &App) -> io::Result<()>{
    let app = CliApp::new(app);
    app.print()?;
    Ok(())
}

pub struct CliApp<'a> {
    todo_app: &'a App,
}

impl <'a>CliApp <'a>{
    #[inline]
    pub fn new(app: &'a App) -> Self {
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
            let mut print_todo = PrintTodoTree::new(self.todo_app.args.show_done);
            print_todo.print_list(&self.todo_app.todo_list);
        } else {
            self.print_list()
        }
        Ok(())
    }
}

#[derive(Clone)]
struct PrintTodoTree {
    was_last: Vec<bool>,
    is_last: bool,
    depth: usize,
    show_done: bool,
}

impl PrintTodoTree {
    #[inline]
    pub fn new(show_done: bool) -> Self {
        PrintTodoTree {
            was_last: vec![],
            is_last: false,
            depth: 0,
            show_done,
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
    pub fn print_list(&mut self, todo_list: &TodoList) {
        let mut todos = todo_list.undone.todos.clone();
        if self.show_done {
            todos.extend(todo_list.done.todos.clone())
        }

        for (index, todo) in todos.iter().enumerate() {
            self.is_last = index == todos.len() - 1;
            if self.depth > 0 {
                self.print_indention();
            }
            self.print_todo(todo);

            if let Some(todo_list) = todo.dependency.todo_list() {
                let mut tree_child = self.tree_child();
                tree_child.print_list(todo_list);
            }

            if let Some(note) = todo.dependency.note() {
                self.print_note(note)
            }
        }
    }

    #[inline]
    fn print_todo(&self, todo: &Todo) {
        println!("{}", todo.display(Some(self.show_done)));
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
        for i in 1..depth {
            if self.was_last[i+1] {
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
