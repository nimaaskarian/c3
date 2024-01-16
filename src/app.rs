use std::fs::File;
use std::io::Write;
// vim:fileencoding=utf-8:foldmethod=marker
// std{{{
use std::{io, path::PathBuf};
//}}}
// lib{{{
use arboard::Clipboard;
use ratatui::widgets::*;
// }}}
// mod{{{
use crate::fileio::todo_path;
use crate::todo_list::{TodoList};
use crate::todo_list::todo::Todo;
//}}}


pub struct App {
    pub todo_list: TodoList,
    pub should_quit: bool,
    pub index: usize,
    todo_path: PathBuf, pub changed:bool,
    pub show_right:bool,
    prior_indexes: Vec<usize>,
    pub text_mode: bool,
    pub on_submit: Option<fn(String, &mut App)->()>,
    pub clipboard: Option<Clipboard>,
    pub potato: bool,
    pub include_done: bool,
    search_indexes: Vec<usize>,
}

impl App {

    #[inline]
    pub fn new() -> Self {
        let clipboard = match Clipboard::new() {
            Ok(some) => Some(some),
            Err(_) => None,
        };
        let todo_path = todo_path().unwrap();
        let mut todo_list = TodoList::read(&todo_path);
        App {
            include_done: false,
            clipboard,
            on_submit: None,
            todo_list,
            should_quit: false,
            prior_indexes: Vec::new(),
            search_indexes: Vec::new(),
            index: 0,
            todo_path,
            changed: false,
            show_right: true,
            text_mode: false,
            potato: false,
        }
    }

    pub fn fix_done_undone(&mut self) {
        self.mut_todo().unwrap().dependencies.fix_undone();
        self.mut_current_list().fix_undone();

        if self.include_done {
            self.mut_todo().unwrap().dependencies.fix_done();
            self.mut_current_list().fix_done();
        }

        if self.is_undone_empty() {
            self.traverse_up();
            match self.mut_todo() {
                Some(todo) => {
                    todo.toggle_done()
                }
                _ => {}
            }
            self.mut_current_list().fix_undone();
            if self.include_done {
                self.mut_current_list().fix_done();
            }
        }
    }
    
    // pub fn search(&mut self, query:String) {
    //     for item in self.current_list() {

    //     }
    //     self.search_indexes
    // }

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
        let todo_string = format!("Todos ({}){changed_str}", self.current_list().len());
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
    pub fn top(&mut self) -> usize{
        0
    }

    #[inline]
    pub fn go_top(&mut self) {
        self.index = self.top();
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
        if self.prior_indexes.len() != 0 {
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
        self.current_list().is_empty()
    }

    pub fn is_undone_empty(&self) -> bool{
        self.current_list().undone.is_empty()
    }

    #[inline]
    pub fn set_text_mode(&mut self, on_submit:fn(String, &mut App)->()) {
        self.on_submit = Some(on_submit);
        self.text_mode = true;
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

    pub fn display(&self) -> Vec<String> {
        self.current_list().display(self.include_done)
    }

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
}
