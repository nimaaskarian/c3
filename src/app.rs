// vim:fileencoding=utf-8:foldmethod=marker
// std{{{
use std::{io::{self, stdout, Write}, path::PathBuf, fs::File};
//}}}
// lib{{{
use ratatui::{prelude::*, widgets::*};
// }}}
// mod{{{
use crate::fileio::todo_path;
use crate::todo_list::{TodoList, TodoArray};
use crate::todo_list::todo::Todo;
//}}}


pub struct App {
    pub todo_list: TodoList,
    pub should_quit: bool,
    pub index: usize,
    todo_path: PathBuf,
    pub changed:bool,
    pub show_note:bool,
    prior_indexes: Vec<usize>,
    pub text_mode: bool,
    pub on_submit: Option<fn(String, &mut App)->()>,
}

impl App {
    pub fn new() -> Self {
        let todo_path = todo_path().unwrap();
        App {
            on_submit: None,
            todo_list: TodoList::read(&todo_path),
            should_quit: false,
            prior_indexes: Vec::new(),
            index: 0,
            todo_path,
            changed: false,
            show_note: true,
            text_mode: false,
        }
    }

    pub fn title(&self) -> String {
        let changed_str = if self.changed {
            "*"
        } else {
            ""
        };
        let todo_string = format!("Todos ({}){changed_str}", self.current_list().undone.len());
        let depth = self.prior_indexes.len();
        
        if depth == 0 {
            todo_string
        } else {
            format!("{todo_string} Depth: {depth}")
        }
    }

    pub fn increment(&mut self) {
        if self.index != self.current_list().undone.len() - 1 {
            self.index += 1
        } else {
            self.go_top()
        }
    }

    pub fn decrement(&mut self) {
        if self.index != 0 {
            self.index -= 1;
        } else {
            self.go_bottom()
        }
    }

    pub fn top(&mut self) -> usize{
        0
    }

    pub fn go_top(&mut self) {
        self.index = self.top();
    }

    pub fn traverse_down(&mut self) {
        if self.todo().unwrap().has_dependency() {
            self.prior_indexes.push(self.index);
            self.index = 0;
        }
    }

    pub fn traverse_up(&mut self) {
        if self.prior_indexes.len() != 0 {
            self.index = self.prior_indexes.remove(self.prior_indexes.len()-1);
        }
    }

    pub fn go_bottom(&mut self) {
        self.index = self.bottom();
    }

    pub fn bottom(&mut self) -> usize {
        match self.current_list().undone.len() {
            0=>0,
            length=>length-1,
        }
    }

    pub fn page_up(&mut self, list_state:&ListState) {
        self.index = list_state.offset();
    }

    pub fn current_undone_empty(&self) -> bool{
        self.current_list().undone.len() == 0
    }

    pub fn set_text_mode(&mut self, on_submit:fn(String, &mut App)->()) {
        self.on_submit = Some(on_submit);
        self.text_mode = true;
    }

    pub fn mut_todo(&mut self) -> Option<&mut Todo> {
        if self.current_undone_empty() {
            return None
        }

        self.changed = true;
        let current_list = self.current_list();
        let index = self.index.min(current_list.undone.len() - 1);
        let undone_len = current_list.undone.len();

        if undone_len <= index {
            return Some(&mut self.mut_current_list().undone[undone_len - 1]);
        }

        Some(&mut self.mut_current_list().undone[index])
    }


    pub fn todo(&self) -> Option<&Todo> {
        if self.current_undone_empty() {
            return None
        }

        let current_list = self.current_list();
        let index = self.index.min(current_list.undone.len() - 1);
        let undone_len = current_list.undone.len();

        if undone_len <= index {
            return Some(&current_list.undone[undone_len - 1]);
        }

        Some(&self.current_list().undone[index])
    }

    pub fn current_list(&self) -> &TodoList {
        let mut list = &self.todo_list;
        if self.prior_indexes.len() == 0 {
            return list;
        }
        for index in self.prior_indexes.iter() {
            list = &list.undone[*index].dependencies
        };
        list
    }

    pub fn mut_current_list(&mut self) -> &mut TodoList {
        self.changed = true;
        let mut list = &mut self.todo_list;
        if self.prior_indexes.len() == 0 {
            return list;
        }
        for index in self.prior_indexes.iter() {
            list = &mut list.undone[*index].dependencies
        };
        list
    }

    pub fn write(&mut self) -> io::Result<()> {
        self.changed = false;
        self.todo_list.write(&self.todo_path)?;
        Ok(())
    }
}
