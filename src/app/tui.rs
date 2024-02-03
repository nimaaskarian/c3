// vim:fileencoding=utf-8:foldmethod=marker
// std {{{
use std::io::{self, stdout};
// }}}
// lib {{{
use crossterm::{
    ExecutableCommand,
    terminal::{disable_raw_mode, LeaveAlternateScreen, enable_raw_mode, EnterAlternateScreen}
};
use ratatui::{prelude::*, widgets::*};
use crate::Args;
//}}}
// mod {{{
mod app;
use modules::potato::Potato;
use super::todo::App;
// }}}

pub fn default_block<'a, T>(title: T) -> Block<'a> 
where
    T: Into<Line<'a>>,
{
    Block::default().title(title).borders(Borders::ALL)
}


pub enum TodoWidget<'a> {
    List(ratatui::widgets::List<'a>),
    Paragraph(ratatui::widgets::Paragraph<'a>),
}

pub fn create_todo_widget(display_list:&Vec<String>, title:String) ->  TodoWidget {
    if display_list.is_empty() {
        return TodoWidget::Paragraph(Paragraph::new("No todo.").block(default_block(title)))
    }
    return TodoWidget::List(List::new((*display_list).clone())
        .block(default_block(title))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true));

}

pub fn shutdown() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(crossterm::cursor::Show)?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

pub fn startup() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Ok(())
}

#[inline]
pub fn restart(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    terminal.clear()?;
    startup()?;
    Ok(())
}

#[inline]
pub fn run(app:&mut App) -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut potato_module = Potato::new(None);
    let mut list_state = ListState::default();
    let mut app = TuiApp::new(app, &mut potato_module);

    loop {
        terminal.draw(|frame| {
            app.ui(frame, &mut list_state)
        })?;

        let operation = app.update_return_operation()?;
        match operation {
            Operation::Restart => restart(&mut terminal)?,
            Operation::Nothing =>{},
        }
    }
}

// vim:fileencoding=utf-8:foldmethod=marker
// lib{{{
use tui_textarea::{Input, TextArea, CursorMove};
use ratatui::{prelude::*, widgets::*};
use crossterm::event::{self, Event::Key, KeyCode::Char, KeyCode};
// }}}
// mod {{{
mod modules;
use modules::Module;
//}}}


#[derive(Debug)]
pub enum Operation {
    Nothing,
    Restart,
}

pub struct TuiApp<'a>{
    show_right:bool,
    text_mode: bool,
    on_submit: Option<fn(&mut Self, String)->()>,
    module_enabled: bool,
    module: &'a mut dyn Module<'a>,
    textarea: TextArea<'a>,
    todo_app: &'a mut App,
}

impl<'a>TuiApp<'a>{

    #[inline]
    pub fn new(app:&'a mut App,module: &'a mut dyn Module<'a>) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        TuiApp {
            todo_app: app,
            textarea,
            module,
            on_submit: None,
            show_right: true,
            text_mode: false,
            module_enabled: false,
        }
    }


    #[inline]
    pub fn title(&self) -> String { 
        let changed_str = if self.todo_app.is_changed() {
            "*"
        } else {
            ""
        };
        let size = self.todo_app.len();
        let todo_string = format!("Todos ({size}){changed_str}");
        
        if self.todo_app.is_root() {
            todo_string
        } else {
            format!("{todo_string} {}", self.todo_app.parent().unwrap().message)
        }
    }

    #[inline]
    pub fn quit(&self) -> io::Result<()>{
        shutdown()?;
        std::process::exit(0);
    }

    #[inline]
    pub fn set_text_mode(&mut self, on_submit:fn(&mut Self, String)->(),title: &'a str ,placeholder: &str) {
        self.on_submit = Some(on_submit);
        self.textarea.set_placeholder_text(placeholder);
        self.textarea.set_block(default_block(title));
        self.text_mode = true;
    }

    #[inline]
    pub fn search_prompt(&mut self) {
        self.set_text_mode(Self::on_search, "Search todo", "Enter search query")
    }

    #[inline]
    fn on_search(&mut self, str:String) {
        self.todo_app.search(Some(str));
        self.todo_app.search_next_index();
    }


    #[inline]
    pub fn schedule_prompt(&mut self) {
        self.set_text_mode(Self::on_schedule, "Change schedule day", "");
    }

    #[inline]
    fn on_schedule(&mut self,str:String) {
        let day = str.parse::<u64>().ok();
        if let Some(todo) = self.todo_app.mut_todo() {
            todo.enable_day(day.unwrap() as i64);
        }
    }


    #[inline]
    pub fn edit_prompt(&mut self, start: bool) {
        let todo_message = match self.todo_app.get_message() {
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
        self.set_text_mode(Self::on_append_todo, "Add todo", "Enter the todo message");
    }

    #[inline]
    pub fn append_prompt(&mut self) {
        self.set_text_mode(Self::on_prepend_todo, "Add todo at first", "Enter the todo message");
    }

    #[inline]
    pub fn quit_save_prompt(&mut self) {
        if self.todo_app.is_changed() {
            self.set_text_mode(Self::on_save_prompt, "You have done changes. You wanna save? [n: no, y: yes, c: cancel] (default: n)", "N/y/c");
        } else {
            self.quit();
        }
    }

    #[inline]
    fn on_save_prompt(app:&mut TuiApp, str:String) {
        let lower = str.to_lowercase();
        if lower.starts_with("y") {
            app.todo_app.write();
        } else if lower.starts_with("c") {
            return;
        }
        app.quit();
    }

    #[inline]
    fn on_append_todo(app: &mut Self, str:String) {
        app.todo_app.append(str);
    }

    #[inline]
    fn on_prepend_todo(app:&mut TuiApp,str:String) {
        app.todo_app.prepend(str);
    }

    #[inline]
    fn on_edit_todo(&mut self,str:String) {
        if !str.is_empty() {
            self.todo_app.mut_todo().unwrap().set_message(str);
        }
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
            self.todo_app.fix_index();
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
                    Char('x') => self.todo_app.cut_todo(),
                    Char('d') => self.todo_app.toggle_current_daily(),
                    Char('W') => self.todo_app.toggle_current_weekly(),
                    Char('S') => self.schedule_prompt(),
                    Char('!') => self.todo_app.toggle_show_done(),
                    Char('y') => self.todo_app.yank_todo(),
                    Char('p') => self.todo_app.paste_todo(),
                    KeyCode::Down | Char('j') => self.todo_app.increment(),
                    KeyCode::Up |Char('k') => self.todo_app.decrement(),
                    KeyCode::Right | Char('l') => self.todo_app.add_dependency_traverse_down(),
                    KeyCode::Left | Char('h') => self.todo_app.traverse_up(),
                    KeyCode::Home | Char('g') => self.todo_app.go_top(),
                    KeyCode::End | Char('G') => self.todo_app.go_bottom(),
                    Char('w') => self.todo_app.write()?,
                    Char('J') => self.todo_app.decrease_current_priority(),
                    Char('K') => self.todo_app.increase_current_priority(),
                    Char(']') => {
                        self.show_right = !self.show_right
                    },
                    Char('P') => {
                        self.module_enabled = !self.module_enabled
                    },
                    Char('>') => {
                        self.todo_app.edit_or_add_note();
                        return Ok(Operation::Restart)
                    },
                    Char('t') => self.todo_app.add_dependency(),
                    Char('D') => {
                        self.todo_app.delete_todo();
                    }
                    Char('R') => self.todo_app.read(),
                    Char('T') => self.todo_app.remove_current_dependent(),
                    KeyCode::Enter => self.todo_app.toggle_current_done(),
                    Char('n') => self.todo_app.search_next(),
                    Char('N') => self.todo_app.search_prev(),
                    Char('a') => self.prepend_prompt(),
                    Char('/') => self.search_prompt(),
                    Char('A') => self.append_prompt(),
                    Char('E') | Char('e') => self.edit_prompt(key.code == Char('E')),
                    Char('q') => self.quit_save_prompt(),
                    Char(c) if c.is_digit(10) => self.todo_app.set_current_priority(c.to_digit(10).unwrap() as i8),

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
        let todo = self.todo_app.todo();

        list_state.select(Some(self.todo_app.index()));
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


        match create_todo_widget(&self.todo_app.display(), self.title()) {
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
                match create_todo_widget(&todo_list.display(self.todo_app.show_done()), String::from("Todo dependencies")) {
                    TodoWidget::List(widget) =>frame.render_widget(widget, todos_layout[1]),
                    TodoWidget::Paragraph(widget) =>frame.render_widget(widget, todos_layout[1]),
                }
            } 
        }
    }
}
