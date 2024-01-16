// vim:fileencoding=utf-8:foldmethod=marker
// std{{{
use std::{io::{self, stdout}, path::PathBuf};
//}}}
// lib{{{
use arboard::Clipboard;
use crossterm::{
    ExecutableCommand,
    terminal::{disable_raw_mode, LeaveAlternateScreen}
};
// }}}
// mod {{{
use crate::fileio::todo_path;
use crate::todo_list::TodoList;
use crate::todo_list::todo::Todo;
//}}}

fn shutdown() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

pub struct App {
    pub todo_list: TodoList,
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
    search_index: usize,
    last_query: String,
}

impl App {

    #[inline]
    pub fn new() -> Self {
        let clipboard = match Clipboard::new() {
            Ok(some) => Some(some),
            Err(_) => None,
        };
        let todo_path = todo_path().unwrap();
        let todo_list = TodoList::read(&todo_path);
        App {
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
            potato: false,
        }
    }

    #[inline]
    pub fn quit(&self) -> io::Result<()>{
        shutdown()?;
        std::process::exit(0);
    }

    #[inline]
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
        while self.is_todos_empty() && !self.prior_indexes.is_empty() {
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
}
