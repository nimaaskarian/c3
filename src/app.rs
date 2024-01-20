// vim:fileencoding=utf-8:foldmethod=marker
// std{{{
use std::{io::{self, stdout}, path::PathBuf};
//}}}
// lib{{{
use tui_textarea::{Input, TextArea, CursorMove};
use ratatui::prelude::*;
use arboard::Clipboard;
use crossterm::{
    event::{self, Event::Key, KeyCode::Char, KeyCode},
    ExecutableCommand,
    terminal::{enable_raw_mode, EnterAlternateScreen, disable_raw_mode, LeaveAlternateScreen}
};
// }}}
// mod {{{
use crate::fileio::todo_path;
use crate::todo_list::TodoList;
use crate::todo_list::todo::Todo;
use crate::modules::Module;
use crate::tui::default_block;
//}}}

fn shutdown() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(crossterm::cursor::Show)?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

pub struct App<'a>{
    pub todo_list: TodoList,
    pub index: usize,
    todo_path: PathBuf, pub changed:bool,
    pub show_right:bool,
    prior_indexes: Vec<usize>,
    pub text_mode: bool,
    pub on_submit: Option<fn(&mut Self, String)->()>,
    pub clipboard: Option<Clipboard>,
    pub module_enabled: bool,
    pub include_done: bool,
    search_indexes: Vec<usize>,
    search_index: usize,
    last_query: String,
    pub module: &'a mut dyn Module<'a>,
    pub textarea: TextArea<'a>,
}

impl<'a>App<'a>{

    #[inline]
    pub fn new(module: &'a mut dyn Module<'a>) -> Self {
        let clipboard = match Clipboard::new() {
            Ok(some) => Some(some),
            Err(_) => None,
        };
        let todo_path = todo_path().unwrap();
        let todo_list = TodoList::read(&todo_path);
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        App {
            textarea,
            module,
            last_query: String::new(),
            search_index: 0,
            include_done: false,
            clipboard,
            on_submit: None,
            todo_list,
            prior_indexes: Vec::new(),
            search_indexes: Vec::new(),
            index: 0,
            todo_path,
            changed: false,
            show_right: true,
            text_mode: false,
            module_enabled: false,
        }
    }

    #[inline]
    pub fn quit(&self) -> io::Result<()>{
        shutdown()?;
        std::process::exit(0);
    }
    
    #[inline]
    pub fn only_undone_empty(&self) -> bool {
        self.is_undone_empty() && !self.is_done_empty()
    }

    #[inline]
    pub fn fix_done_undone(&mut self) {
        self.mut_todo().unwrap().dependencies.fix_undone();
        self.mut_current_list().fix_undone();

        if self.include_done {
            self.mut_todo().unwrap().dependencies.fix_done();
            self.mut_current_list().fix_done();
        }

        while self.only_undone_empty() && !self.prior_indexes.is_empty() {
            self.traverse_up();
            match self.mut_todo() {
                Some(todo) => {
                    todo.set_done(true)
                }
                _ => {}
            }
            self.mut_current_list().fix_undone();
            if self.include_done {
                self.mut_current_list().fix_done();
            }
        }
    }
    
    #[inline]
    pub fn search(&mut self, query:Option<String>) {
        if let Some(query) = query {
            self.last_query = query;
        }
        if self.last_query.is_empty() {
            return;
        }
        let mut todo_messages = self.current_list().undone.messages();
        if self.include_done {
            todo_messages.extend(self.current_list().done.messages());
        }
        self.search_indexes = Vec::new();

        for i in 0..todo_messages.len() {
            if todo_messages[i].to_lowercase().contains(self.last_query.to_lowercase().as_str()) {
                self.search_indexes.push(i);
            }
        }
    }

    #[inline]
    pub fn search_next_index(&mut self) {
        if self.search_indexes.is_empty() {
            return;
        }
        for index in &self.search_indexes {
            if *index > self.index{
                self.index = *index;
                return;
            }
        }

        self.index = self.search_indexes[0];
    }

    pub fn toggle_include_done(&mut self) {
        self.include_done = !self.include_done;
        while self.only_undone_empty() && !self.prior_indexes.is_empty() {
            self.traverse_up()
        }
        self.search(None);
    }

    pub fn search_next(&mut self) {
        if self.search_indexes.is_empty() {
            return;
        }
        if self.search_index+1 < self.search_indexes.len() {
            self.search_index+=1
        } else {
            self.search_index=0
        }
        self.index = self.search_indexes[self.search_index]
    }

    pub fn search_prev(&mut self) {
        if self.search_indexes.is_empty() {
            return;
        }
        if self.search_index != 0 {
            self.search_index-=1
        } else {
            self.search_index=self.search_indexes.len()-1
        }
        self.index = self.search_indexes[self.search_index]
    }

    pub fn toggle_current_done(&mut self) {
        let was_done = self.todo().unwrap().done();
        self.mut_todo().unwrap().toggle_done();
        self.fix_done_undone();
        let index = if was_done {
            self.current_list().undone.len()-1
        } else {
            self.current_list().len()-1
        };
        self.index = self.mut_current_list().reorder(index);
    }

    #[inline]
    pub fn read(&mut self) {
        self.changed = false;
        self.todo_list = TodoList::read(&self.todo_path);
    }

    #[inline]
    pub fn title(&self) -> String { let changed_str = if self.changed {
            "*"
        } else {
            ""
        };
        let size = if self.include_done {
            self.current_list().len()
        } else {
            self.current_list().undone.len()
        };
        let todo_string = format!("Todos ({size}){changed_str}");
        let depth = self.prior_indexes.len();
        
        if depth == 0 {
            todo_string
        } else {
            format!("{todo_string} {}", self.parent().unwrap().message)
        }
    }

    #[inline]
    pub fn parent(&self) -> Option<&Todo>{
        let mut list = &self.todo_list;
        let mut parent = None;
        for index in self.prior_indexes.iter() {
            parent = Some(&list[*index]);
            list = &list[*index].dependencies
        };
        parent
    }

    #[inline]
    pub fn increment(&mut self) {
        let size = self.len();
        if size == 0 {
            self.index = 0;
            return;
        };
        if self.index != size - 1 {
            self.index += 1
        } else {
            self.go_top()
        }
    }

    #[inline]
    pub fn decrement(&mut self) {
        if self.index != 0 {
            self.index -= 1;
        } else {
            self.go_bottom()
        }
    }

    #[inline]
    pub fn go_top(&mut self) {
        self.index = 0;
    }

    #[inline]
    pub fn traverse_down(&mut self) {
        match self.todo() {
            Some(todo) if todo.has_dependency() => {
                self.prior_indexes.push(self.index);
                self.index = 0;
            }
            _ => {},
        }
    }

    #[inline]
    pub fn traverse_up(&mut self) {
        if !self.prior_indexes.is_empty() {
            self.index = self.prior_indexes.remove(self.prior_indexes.len()-1);
        }
    }

    #[inline]
    pub fn go_bottom(&mut self) {
        self.index = self.bottom();
    }

    #[inline]
    pub fn bottom(&mut self) -> usize {
        match self.len() {
            0=>0,
            length=>length-1,
        }
    }

    #[inline]
    pub fn is_todos_empty(&self) -> bool{
        if self.include_done {
            self.current_list().is_empty()
        } else {
            self.is_undone_empty()
        }
    }

    #[inline]
    pub fn is_undone_empty(&self) -> bool{
        self.current_list().undone.is_empty()
    }

    #[inline]
    pub fn is_done_empty(&self) -> bool{
        self.current_list().done.is_empty()
    }

    #[inline]
    pub fn set_text_mode(&mut self, on_submit:fn(&mut Self, String)->(),title: &'a str ,placeholder: &str) {
        self.on_submit = Some(on_submit);
        self.textarea.set_placeholder_text(placeholder);
        self.textarea.set_block(default_block(title));
        self.text_mode = true;
    }

    #[inline]
    pub fn set_current_priority(&mut self, priority:i8) {
        if let Some(todo) = self.mut_todo() {
            todo.set_priority(priority);
            self.reorder_current();
        }
    }

    #[inline]
    pub fn search_prompt(&mut self) {
        self.set_text_mode(Self::on_search, "Search todo", "Enter search query")
    }

    #[inline]
    fn on_search(&mut self, str:String) {
        self.search(Some(str));
        self.search_next_index();
    }


    #[inline]
    pub fn get_message(&mut self) -> Option<String> {
        if let Some(todo) = self.todo() {
            return Some(todo.message.clone())
        };
        None
    }

    #[inline]
    pub fn edit_prompt(&mut self, start: bool) {
        let todo_message = match self.get_message() {
            Some(message)=>message,
            None=> String::new(),
        };

        self.set_text_mode(Self::on_edit_todo, "Edit todo", todo_message.as_str());
        self.textarea.insert_str(todo_message);
        if start {
            self.textarea.move_cursor(CursorMove::Head);
        }
    }

    #[inline]
    pub fn prepend_prompt(&mut self) {
        self.set_text_mode(Self::on_prepend_todo, "Add todo", "Enter the todo message");
    }

    #[inline]
    pub fn append_prompt(&mut self) {
        self.set_text_mode(Self::on_append_todo, "Add todo at first", "Enter the todo message");
    }

    #[inline]
    pub fn quit_save_prompt(&mut self) {
        if self.changed {
            self.set_text_mode(Self::on_save_prompt, "You have done changes. You wanna save em? [n: no, y: yes, c: cancel]", "N/y/c");
        } else {
            self.quit();
        }

    }

    #[inline]
    fn on_save_prompt(app:&mut App, str:String) {
        let lower = str.to_lowercase();
        if lower.starts_with("y") {
            app.write();
        } else if lower.starts_with("c") {
            return;
        }
        app.quit();
    }

    #[inline]
    fn on_prepend_todo(app: &mut Self, str:String) {
        app.mut_current_list().push(Todo::new(str, 0));
        app.index = app.current_list().undone.len()-1;
    }

    #[inline]
    fn on_append_todo(app:&mut App,str:String) {
        app.mut_current_list().prepend(Todo::new(str, 1));
        app.index = 0;
    }

    #[inline]
    fn on_edit_todo(&mut self,str:String) {
        if !str.is_empty() {
            self.mut_todo().unwrap().set_message(str);
    }
}


    #[inline]
    pub fn mut_todo(&mut self) -> Option<&mut Todo> {
        if self.is_todos_empty() {
            return None
        }

        // let current_list = self.current_list();
        let index = self.index.min(self.len() - 1);
        let size = self.len();

        if size <= index {
            return Some(&mut self.mut_current_list()[size - 1]);
        }

        Some(&mut self.mut_current_list()[index])
    }

    #[inline]
    pub fn cut_todo(&mut self) {
        if !self.is_todos_empty() {
            let index = self.index;
            let todo = self.mut_current_list().remove(index);
            let todo_string:String = (&todo).into();
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(todo_string);
            }
        }
    }

    #[inline]
    pub fn add_dependency_traverse_down(&mut self) {
        if let Some(todo) = self.todo() {
            if !todo.has_dependency() && todo.note_empty() {
                self.mut_todo().unwrap().add_dependency();
            }
        }
        self.traverse_down()
    }

    #[inline]
    pub fn yank_todo(&mut self) {
        let todo_string:String = self.todo().unwrap().into();
        if let Some(clipboard) = &mut self.clipboard {
            let _ = clipboard.set_text(todo_string);
        }
    }

    #[inline]
    pub fn paste_todo(&mut self) {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                match Todo::try_from(text) {
                    Ok(todo) => {
                        let list = &mut self.mut_current_list();
                        list.push(todo);
                        list.reorder(list.len());
                    },
                    _ => {},
                };
            }
        }
    }
    
    #[inline]
    pub fn increase_current_priority(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.increase_priority();
            self.reorder_current();
        }
    }

    #[inline]
    pub fn decrease_current_priority(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.decrease_priority();
            self.reorder_current();
        }
    }

    #[inline]
    pub fn edit_or_add_note(&mut self) {
        if let Some(todo) = self.mut_todo() {
            if todo.edit_note().is_err() {
                todo.add_note();
            }
        }
    }

    #[inline]
    pub fn add_dependency(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.add_dependency();
        }
    }

    pub fn remove_current_dependent(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.remove_note();
            todo.remove_dependency();
        }
    }

    pub fn delete_todo(&mut self) {
        if !self.is_todos_empty() {
            let index = self.index;
            self.mut_current_list().undone.remove(index);
        }
    }

    #[inline]
    pub fn reorder_current(&mut self) {
        let index = self.index;
        self.index = self.mut_current_list().reorder(index);
    }

    #[inline]
    pub fn todo(&self) -> Option<&Todo> {
        if self.is_todos_empty() {
            return None
        }

        let current_list = self.current_list();
        let index = self.index.min(self.len() - 1);
        let size = self.len();

        if size <= index {
            return Some(&current_list[size - 1]);
        }

        Some(&self.current_list()[index])
    }


    #[inline]
    pub fn current_list(&self) -> &TodoList {
        let mut list = &self.todo_list;
        if self.prior_indexes.len() == 0 {
            return list;
        }
        for index in self.prior_indexes.iter() {
            list = &list[*index].dependencies
        };
        list
    }

    #[inline]
    pub fn display(&self) -> Vec<String> {
        self.current_list().display(self.include_done)
    }

    #[inline]
    pub fn len(&self) -> usize {
        if self.include_done {
            self.current_list().len()
        } else {
            self.current_list().undone.len()
        }
    }

    #[inline]
    pub fn mut_current_list(&mut self) -> &mut TodoList {
        self.changed = true;
        let mut list = &mut self.todo_list;
        if self.prior_indexes.len() == 0 {
            return list;
        }
        for index in self.prior_indexes.iter() {
            list = &mut list[*index].dependencies
        };
        list
    }

    #[inline]
    pub fn write(&mut self) -> io::Result<()> {
        self.changed = false;
        self.todo_list.write(&self.todo_path)?;
        Ok(())
    }

    #[inline]
    fn enable_text_editor(&mut self) -> io::Result<()>{
        match self.editor()? {
            None => {},
            Some(should_add) => {
                if should_add {
                    let todo_message = self.textarea.lines()[0].clone();
                    self.on_submit.unwrap()(self, todo_message);
                }
                self.textarea.delete_line_by_head();
                self.textarea.delete_line_by_end();
                self.text_mode = false;
            }
        }
        Ok(())
    }

    #[inline]
    pub fn update_text_editor(&mut self)  -> io::Result<()> {
        if self.module_enabled {
            if event::poll(std::time::Duration::from_millis(500))? {
                return self.enable_text_editor()
            }
        } else {
            return self.enable_text_editor()
        }
        Ok(())
    }

    #[inline]
    fn editor(&mut self) -> io::Result<Option<bool>> {
        match crossterm::event::read()?.into() {
            Input {
                key: tui_textarea::Key::Esc, .. 
            } => Ok(Some(false)),
            Input {
                key: tui_textarea::Key::Enter, ..
            }=> Ok(Some(true)),
            Input {
                key: tui_textarea::Key::Char('u'),
                ctrl: true,
                ..
            } => {
                self.textarea.delete_line_by_head();
                Ok(None)
            },
            input => {
                self.textarea.input(input) ;
                Ok(None)
            }
        }
    }
}
