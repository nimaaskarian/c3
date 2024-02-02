// vim:fileencoding=utf-8:foldmethod=marker
// std{{{
use std::{io, path::PathBuf};
//}}}
// lib{{{
use tui_textarea::{Input, TextArea, CursorMove};
use ratatui::{prelude::*, widgets::*};
use crossterm::event::{self, Event::Key, KeyCode::Char, KeyCode};
// }}}
// mod {{{
use crate::tui::{default_block, create_todo_widget, TodoWidget, shutdown, modules::Module};
use crate::fileio::todo_path;
use crate::todo_list::TodoList;
use crate::todo_list::todo::Todo;
use crate::Args;
use super::clipboard::Clipboard;
//}}}


#[derive(Debug)]
pub enum Operation {
    Nothing,
    Restart,
}

pub struct App<'a>{
    todo_list: TodoList,
    index: usize,
    todo_path: PathBuf,
    pub changed:bool,
    show_right:bool,
    prior_indexes: Vec<usize>,
    text_mode: bool,
    on_submit: Option<fn(&mut Self, String)->()>,
    clipboard: Clipboard,
    module_enabled: bool,
    show_done: bool,
    search_indexes: Vec<usize>,
    search_index: usize,
    last_query: String,
    module: &'a mut dyn Module<'a>,
    textarea: TextArea<'a>,
}

impl<'a>App<'a>{

    #[inline]
    pub fn new(args:Args,module: &'a mut dyn Module<'a>) -> Self {
        let clipboard = Clipboard::new();
        let todo_path = match args.todo_path {
            Some(path) => path,
            None => todo_path().unwrap(),
        };
        let todo_list = TodoList::read(&todo_path, true, true);
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        App {
            textarea,
            module,
            last_query: String::new(),
            search_index: 0,
            show_done: args.show_done,
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
        self.fix_dependency_done_undone();
        let show_done = self.show_done;
        let current_list = self.mut_current_list();
        current_list.fix_undone();
        if show_done {
            current_list.fix_done();
        }

        self.traverse_up_and_fix();
    }

    #[inline]
    fn fix_dependency_done_undone(&mut self) {
        let show_done = self.show_done;
        if let Some(todo) = self.mut_todo() {

            let dep_list = &mut todo.dependency.todo_list;

            dep_list.fix_undone();
            if show_done {
                dep_list.fix_done();
            }

        }

    }

    #[inline]
    fn traverse_up_and_fix(&mut self) {
        while self.only_undone_empty() && !self.is_root() {
            self.traverse_up();
            match self.mut_todo() {
                Some(todo) => {
                    todo.set_done(true)
                }
                _ => {}
            }
            self.mut_current_list().fix_undone();
            if self.show_done {
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
        if self.show_done {
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

    #[inline]
    pub fn toggle_show_done(&mut self) {
        self.show_done = !self.show_done;
        // while self.only_undone_empty() && !self.prior_indexes.is_empty() {
        //     self.traverse_up()
        // }
        self.search(None);
    }

    #[inline]
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

    #[inline]
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

    #[inline]
    pub fn toggle_current_done(&mut self) {
        let was_done = self.todo().unwrap().done();
        self.mut_todo().unwrap().toggle_done();
        self.fix_done_undone();
        if self.show_done {
            let index = if was_done {
                self.current_list().undone.len()-1
            } else {
                self.current_list().len()-1
            };
            self.index = self.mut_current_list().reorder(index);
        }
    }

    #[inline]
    pub fn toggle_current_daily(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.toggle_daily()
        }
    }

    #[inline]
    pub fn toggle_current_weekly(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.toggle_weekly()
        }
    }

    #[inline]
    pub fn read(&mut self) {
        self.changed = false;
        self.todo_list = TodoList::read(&self.todo_path, true, true);
    }

    #[inline]
    pub fn title(&self) -> String { 
        let changed_str = if self.changed {
            "*"
        } else {
            ""
        };
        let size = self.len();
        let todo_string = format!("Todos ({size}){changed_str}");
        
        if self.is_root() {
            todo_string
        } else {
            format!("{todo_string} {}", self.parent().unwrap().message)
        }
    }

    #[inline]
    pub fn fix_index(&mut self) {
        let size = self.len();
        self.index = match size {
            0 => 0,
            _ => self.index.min(size-1),
        };
    }

    #[inline]
    pub fn parent(&self) -> Option<&Todo>{
        let mut list = &self.todo_list;
        let mut parent = None;
        for index in self.prior_indexes.iter() {
            parent = Some(&list[*index]);
            if let Some(todo_list) = &list[*index].dependency.todo_list() {
                list = todo_list
            } else {
                break
            }
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
            Some(todo) if todo.dependency.is_list() => {
                self.prior_indexes.push(self.index);
                self.index = 0;
            }
            _ => {},
        }
    }

    #[inline]
    pub fn traverse_up(&mut self) {
        if !self.is_root() {
            self.index = self.prior_indexes.remove(self.prior_indexes.len()-1);
        }
    }

    #[inline]
    pub fn go_bottom(&mut self) {
        self.index = self.bottom();
    }

    #[inline]
    pub fn bottom(&self) -> usize {
        match self.len() {
            0=>0,
            length=>length-1,
        }
    }

    #[inline]
    pub fn is_todos_empty(&self) -> bool{
        if self.show_done {
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
    pub fn schedule_prompt(&mut self) {
        self.set_text_mode(Self::on_schedule, "Change schedule day", "");
    }

    #[inline]
    fn on_schedule(&mut self,str:String) {
        let day = str.parse::<u64>().ok();
        if let Some(todo) = self.mut_todo() {
            todo.enable_day(day.unwrap() as i64);
        }
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
            self.set_text_mode(Self::on_save_prompt, "You have done changes. You wanna save? [n: no, y: yes, c: cancel] (default: n)", "N/y/c");
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
        app.mut_current_list().push(Todo::default(str, 0));
        app.index = app.current_list().undone.len()-1;
    }

    #[inline]
    fn on_append_todo(app:&mut App,str:String) {
        app.mut_current_list().prepend(Todo::default(str, 1));
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
            self.clipboard.set_text(todo_string);
        }
    }

    #[inline]
    pub fn add_dependency_traverse_down(&mut self) {
        if let Some(todo) = self.todo() {
            if todo.dependency.is_none() {
                self.mut_todo().unwrap().add_todo_dependency();
            }
        }
        self.traverse_down()
    }

    #[inline]
    pub fn yank_todo(&mut self) {
        let todo_string:String = self.todo().unwrap().into();
        self.clipboard.set_text(todo_string);
    }

    #[inline]
    pub fn paste_todo(&mut self) {
        match Todo::try_from(self.clipboard.get_text()) {
            Ok(mut todo) => {
                let bottom = self.bottom();
                let todo_parent = TodoList::dependency_parent(&self.todo_path, true);
                todo.dependency.read(&todo_parent);
                let list = &mut self.mut_current_list();
                list.push(todo);
                self.index = list.reorder(bottom);
            },
            _ => {},
        };
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
            if todo.edit_note().is_ok() {
                return;
            }
        }
    }

    #[inline]
    pub fn add_dependency(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.add_todo_dependency();
        }
    }

    #[inline]
    pub fn remove_current_dependent(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.remove_dependency();
        }
    }

    #[inline]
    pub fn delete_todo(&mut self) {
        if !self.is_todos_empty() {
            let index = self.index;
            self.mut_current_list().remove(index);
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
    pub fn is_root(&self) -> bool {
        self.prior_indexes.is_empty()
    }

    #[inline]
    pub fn current_list(&self) -> &TodoList {
        let mut list = &self.todo_list;
        if self.is_root() {
            return list;
        }
        for index in self.prior_indexes.iter() {
            if let Some(todo_list) = &list[*index].dependency.todo_list() {
                list = todo_list
            } else {
                break
            }
        };
        list
    }

    #[inline]
    pub fn display(&self) -> Vec<String> {
        self.current_list().display(self.show_done)
    }

    #[inline]
    pub fn len(&self) -> usize {
        if self.show_done {
            self.current_list().len()
        } else {
            self.current_list().undone.len()
        }
    }

    #[inline]
    pub fn mut_current_list(&mut self) -> &mut TodoList {
        self.changed = true;
        let is_root = self.is_root();
        let mut list = &mut self.todo_list;
        if  is_root{
            return list;
        }
        for index in self.prior_indexes.iter() {
            list = &mut list[*index].dependency.todo_list
        };
        list
    }

    #[inline]
    pub fn write(&mut self) -> io::Result<()> {
        self.changed = false;
        self.todo_list.write(&self.todo_path, true)?;
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
    pub fn update_editor(&mut self)  -> io::Result<Operation> {
        if self.module_enabled {
            if event::poll(std::time::Duration::from_millis(self.module.update_time_ms()))? {
                self.enable_text_editor()?
            }
        } else {
            self.enable_text_editor()?
        }
        Ok(Operation::Nothing)
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

    #[inline]
    pub fn update_return_operation(&mut self) -> io::Result<Operation>{
        let output;
        if self.text_mode {
            output = self.update_editor()?;
        } else {
            output = self.update_no_editor()?;
            self.fix_index();
        }
        Ok(output)
    }


    #[inline]
    fn update_no_editor(&mut self) -> io::Result<Operation> {
        if self.module_enabled {
            if event::poll(std::time::Duration::from_millis(self.module.update_time_ms()))? {
                return self.read_keys();
            }
        } else {
            return self.read_keys();
        }
        Ok(Operation::Nothing)
    }

    #[inline]
    fn read_keys(&mut self)  -> io::Result<Operation> {
        if let Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    Char('x') => self.cut_todo(),
                    Char('d') => self.toggle_current_daily(),
                    Char('W') => self.toggle_current_weekly(),
                    Char('S') => self.schedule_prompt(),
                    Char('!') => self.toggle_show_done(),
                    Char('y') => self.yank_todo(),
                    Char('p') => self.paste_todo(),
                    KeyCode::Down | Char('j') => self.increment(),
                    KeyCode::Up |Char('k') => self.decrement(),
                    KeyCode::Right | Char('l') => self.add_dependency_traverse_down(),
                    KeyCode::Left | Char('h') => self.traverse_up(),
                    KeyCode::Home | Char('g') => self.go_top(),
                    KeyCode::End | Char('G') => self.go_bottom(),
                    Char('w') => self.write()?,
                    Char('J') => self.decrease_current_priority(),
                    Char('K') => self.increase_current_priority(),
                    Char(']') => {
                        self.show_right = !self.show_right
                    },
                    Char('P') => {
                        self.module_enabled = !self.module_enabled
                    },
                    Char('>') => {
                        self.edit_or_add_note();
                        return Ok(Operation::Restart)
                    },
                    Char('t') => self.add_dependency(),
                    Char('D') => {
                        self.delete_todo();
                    }
                    Char('R') => self.read(),
                    Char('T') => self.remove_current_dependent(),
                    KeyCode::Enter => self.toggle_current_done(),
                    Char('n') => self.search_next(),
                    Char('N') => self.search_prev(),
                    Char('a') => self.prepend_prompt(),
                    Char('/') => self.search_prompt(),
                    Char('A') => self.append_prompt(),
                    Char('E') | Char('e') => self.edit_prompt(key.code == Char('E')),
                    Char('q') => self.quit_save_prompt(),
                    Char(c) if c.is_digit(10) => self.set_current_priority(c.to_digit(10).unwrap() as i8),

                    Char(' ') => self.module.on_space(),
                    Char('s') => self.module.on_s(),
                    Char('H') => self.module.on_capital_h(),
                    Char('c') => self.module.on_c(),
                    Char('L') => self.module.on_capital_l(),
                    Char('r') => self.module.on_r(),
                    Char('+') | Char('=') => self.module.on_plus(),
                    Char('-') => self.module.on_minus(),
                    Char('.') => self.module.on_dot(),
                    Char(',') => self.module.on_comma(),
                    _ => {},
                }
            }
        }
        Ok(Operation::Nothing)
    }

    pub fn ui(&self, frame:&mut Frame, list_state: &mut ListState) {
        let todo = self.todo();

        list_state.select(Some(self.index));
        // let note = match (todo, self.show_right) {
        //     (Some(todo), true)  => todo.get_note_content(),
        //     _ => String::new(),
        // };

        let dependency_width = if let Some(todo) = todo {
            let should_show_right = !todo.dependency.is_none() && self.show_right;
            40 * (should_show_right as u16)
        } else {
            0
        };
        let main_layout = if self.module_enabled {
             let main_layout = Layout::new(
                Direction::Vertical,
                [
                    Constraint::Length(5),
                    Constraint::Min(0),
                ]
            ).split(frame.size());
            frame.render_widget(self.module.get_widget(), main_layout[0]);
            main_layout
        } else {
             Layout::new(
                Direction::Vertical,
                [
                    Constraint::Min(0),
                ]
            ).split(frame.size())
        };

        let todos_layout = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Percentage(100 - dependency_width),
                Constraint::Percentage(dependency_width),
            ]
        ).split(main_layout[self.module_enabled as usize]);

        let todo_layout = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(3 * self.text_mode as u16),
                Constraint::Min(0),
            ]
        ).split(todos_layout[0]);


        match create_todo_widget(&self.display(), self.title()) {
            TodoWidget::Paragraph(widget) => frame.render_widget(widget, todo_layout[1]),
            TodoWidget::List(widget) => frame.render_stateful_widget(widget, todo_layout[1], list_state),
        };

        frame.render_widget(self.textarea.widget(), todo_layout[0]);
        
        if todo.is_some() && self.show_right{
            let todo = todo.unwrap();
            if let Some(note) = todo.dependency.note() {
                let note_widget = Paragraph::new(Text::styled(note, Style::default())).wrap(Wrap { trim: true }).block(default_block("Todo note"));
                frame.render_widget(note_widget, todos_layout[1]);
            } else
            if let Some(todo_list) = todo.dependency.todo_list() {
                match create_todo_widget(&todo_list.display(self.show_done), String::from("Todo dependencies")) {
                    TodoWidget::List(widget) =>frame.render_widget(widget, todos_layout[1]),
                    TodoWidget::Paragraph(widget) =>frame.render_widget(widget, todos_layout[1]),
                }
            } 
        }
    }
}
