use std::io;
use crate::DisplayArgs;

use super::todo_app::{App ,TodoArray, Todo};

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
            // let mut print_todo = PrintTodoTree::new(self.todo_app.args.minimal_tree);
            // print_todo.print_list(&self.todo_app.todo_list, &self.todo_app.args.display_args);
        } else {
            self.print_list()
        }
        Ok(())
    }
}
