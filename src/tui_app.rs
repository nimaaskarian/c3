// vim:fileencoding=utf-8:foldmethod=marker
// std {{{
use std::{
    iter,
    io::{self, stdout, BufRead, BufReader},
    path::PathBuf,
    process::{Command, self, Stdio},
    rc::Rc,
};
// }}}
// lib {{{
use crossterm::{
    event::{
        self,
        Event::Key,
        KeyCode::{self, Char},
        KeyModifiers,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use tui_textarea::{CursorMove, Input, TextArea};
// }}}
// mod {{{

mod modules;
use super::todo_app::{App, Todo};
use crate::{todo_app::RestrictionFunction, TuiArgs};
use crate::{date, todo_app::PriorityType};
use modules::{potato::Potato, Module};
// }}}

pub fn default_block<'a, T>(title: T) -> Block<'a>
where
    T: Into<Line<'a>>,
{
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
}

pub enum TodoWidget<'a> {
    List(ratatui::widgets::List<'a>),
    Paragraph(ratatui::widgets::Paragraph<'a>),
}

pub fn create_todo_widget(
    display_list: Vec<String>,
    title: String,
    highlight_symbol: &str,
) -> TodoWidget<'_> {
    if display_list.is_empty() {
        TodoWidget::Paragraph(Paragraph::new("No todo.").block(default_block(title)))
    } else {
        TodoWidget::List(
            List::new(display_list)
                .block(default_block(title))
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol(highlight_symbol)
                .repeat_highlight_symbol(true),
        )
    }
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
pub fn run(app: &mut App, args: TuiArgs) -> io::Result<()> {
    startup()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut potato_module = Potato::new(None);
    let mut list_state = ListState::default();
    let mut app = TuiApp::new(app, &mut potato_module, args);

    loop {
        terminal.draw(|frame| app.ui(frame, &mut list_state))?;

        let operation = app.update_return_operation()?;
        match operation {
            Operation::Restart => restart(&mut terminal)?,
            Operation::Nothing => {}
        }
    }
}

#[derive(Debug)]
pub enum Operation {
    Nothing,
    Restart,
}

#[derive(Debug, PartialEq)]
enum EditorOperation {
    Cancel,
    Submit,
    Input,
    Delete(String),
}

type HandlerParameter = String;
pub struct TuiApp<'a> {
    last_restriction: Option<RestrictionFunction>,
    show_right: bool,
    text_mode: bool,
    on_submit: Option<fn(&mut Self, HandlerParameter) -> ()>,
    on_delete: Option<fn(&mut Self, HandlerParameter, String) -> ()>,
    on_input: Option<fn(&mut Self, HandlerParameter) -> ()>,
    args: TuiArgs,
    module: &'a mut dyn Module<'a>,
    textarea: TextArea<'a>,
    todo_app: &'a mut App,
}

impl<'a> TuiApp<'a> {
    #[inline]
    pub fn new(app: &'a mut App, module: &'a mut dyn Module<'a>, args: TuiArgs) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        TuiApp {
            todo_app: app,
            textarea,
            module,
            on_submit: None,
            on_input: None,
            on_delete: None,
            show_right: true,
            text_mode: false,
            args,
            last_restriction: None,
        }
    }

    #[inline]
    pub fn title(&mut self) -> String {
        let changed_str = if self.todo_app.is_current_changed() { "*" } else { "" };
        let size = self.todo_app.len();
        let todo_string = format!("Todos ({size}){changed_str}");

        if self.todo_app.is_root() {
            todo_string
        } else {
            format!("{todo_string} {}", self.todo_app.parent().unwrap().message)
        }
    }

    #[inline]
    pub fn quit(&self) -> io::Result<()> {
        shutdown()?;
        std::process::exit(0);
    }

    #[inline]
    pub fn set_text_mode(
        &mut self,
        on_submit: fn(&mut Self, HandlerParameter) -> (),
        title: &'a str,
        placeholder: &str,
    ) {
        self.on_input = None;
        self.on_submit = Some(on_submit);
        self.turn_on_text_mode(title, placeholder);
    }

    #[inline]
    pub fn set_responsive_text_mode(
        &mut self,
        on_input: fn(&mut Self, HandlerParameter) -> (),
        title: &'a str,
        placeholder: &str,
    ) {
        self.on_input = Some(on_input);
        self.turn_on_text_mode(title, placeholder);
    }

    #[inline(always)]
    fn turn_on_text_mode(&mut self, title: &'a str, placeholder: &str) {
        self.textarea.set_placeholder_text(placeholder);
        self.textarea.set_block(default_block(title));
        self.text_mode = true;
    }

    #[inline(always)]
    fn turn_off_text_mode(&mut self) {
        self.textarea.delete_line_by_head();
        self.textarea.delete_line_by_end();
        self.text_mode = false;
    }

    #[inline]
    pub fn search_prompt(&mut self) {
        const TITLE: &str = "Search todo";
        const PLACEHOLDER: &str = "Enter search query";
        self.last_restriction = Some(self.todo_app.restriction().clone());
        self.set_responsive_text_mode(Self::on_search, TITLE, PLACEHOLDER);
        self.on_delete = Some(Self::on_search_delete);
    }

    #[inline]
    pub fn tree_search_prompt(&mut self) {
        self.set_text_mode(
            Self::on_tree_search,
            "Search the whole tree for todo",
            "Enter search query",
        )
    }

    #[inline]
    fn on_search(&mut self, str: String) {
        self.todo_app
            .set_query_restriction(str, self.last_restriction.clone())
    }

    #[inline]
    fn on_priority_delete(&mut self, new: String, old: String) {
        if new.is_empty() {
            if let Some(restriction) = self.last_restriction.clone() {
                self.todo_app.set_restriction(restriction)
            }
        }
        if old.is_empty() {
            self.todo_app.update_show_done_restriction()
        }
    }

    #[inline]
    fn on_search_delete(&mut self, str: String, old: String) {
        if old.is_empty() {
            self.todo_app.update_show_done_restriction()
        } else {
            self.on_search(str)
        }
    }

    #[inline]
    fn on_tree_search(&mut self, str: String) {
        self.todo_app.tree_search(str);
    }

    #[inline]
    pub fn schedule_prompt(&mut self) {
        self.set_text_mode(Self::on_schedule, "Change schedule day", "");
    }

    #[inline]
    fn on_schedule(&mut self, str: HandlerParameter) {
        let day = str.parse::<u64>().ok();
        if day.is_none() {
            return;
        }
        if let Some(todo) = self.todo_app.todo_mut() {
            todo.enable_day(day.unwrap() as i64);
        }
    }

    #[inline]
    pub fn reminder_prompt(&mut self) {
        self.set_text_mode(Self::on_reminder, "Date reminder", "");
    }

    fn nnn_paths() -> Option<iter::Map<io::Lines<BufReader<process::ChildStdout>>, impl FnMut(Result<String, io::Error>) -> PathBuf>> {
        let mut output = Command::new("nnn")
            .args(["-p", "-"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Failed to start nnn.");

        let exit_status = output.wait().expect("Failed to wait on nnn.");
        if exit_status.success() {
            let reader = BufReader::new(output.stdout.unwrap());
            return Some(reader
                .lines()
                .map(|x| PathBuf::from(x.unwrap_or_default())))
        }
        None
    }

    #[inline]
    pub fn nnn_append_todo(&mut self) {
        if let Some(paths) = Self::nnn_paths() {
            for path in paths {
                self.todo_app.append_list_from_path(&path);
            }
        }
    }

    pub fn nnn_open(&mut self) {
        if let Some(paths) = Self::nnn_paths() {
            for path in paths {
                self.todo_app.open_path(path);
            }
        }
    }

    #[inline]
    pub fn nnn_output_todo(&mut self) {
        if let Some(paths) = Self::nnn_paths() {
            for path in paths {
                let _ = self.todo_app.output_list_to_path(&path);
            }
        }
    }

    #[inline]
    fn on_reminder(&mut self, str: HandlerParameter) {
        if let Ok(date) = date::parse_user_input(&str) {
            if let Some(todo) = self.todo_app.todo_mut() {
                todo.schedule.enable_reminder(date);
            }
        }
    }

    #[inline]
    pub fn edit_prompt(&mut self, start: bool) {
        let todo_message = self.todo_app.get_message().unwrap_or_default();

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
    pub fn priority_prompt(&mut self) {
        const TITLE: &str = "Limit priority";
        const PLACEHOLDER: &str = "Enter priority to show";
        self.last_restriction = Some(self.todo_app.restriction().clone());
        self.set_text_mode(Self::on_priority_prompt, TITLE, PLACEHOLDER);
        self.set_responsive_text_mode(Self::on_priority_prompt, TITLE, PLACEHOLDER);
        self.on_delete = Some(Self::on_priority_delete);
    }

    #[inline]
    pub fn append_prompt(&mut self) {
        self.set_text_mode(
            Self::on_prepend_todo,
            "Add todo at first",
            "Enter the todo message",
        );
    }

    #[inline]
    pub fn quit_save_prompt(&mut self) {
        if self.todo_app.is_changed() || self.todo_app.is_current_changed() {
            self.set_text_mode(
                Self::on_save_prompt,
                "You have done changes. You wanna save? [n: no, y: yes, c: cancel] (default: n)",
                "N/y/c",
            );
        } else {
            let _ = self.quit();
        }
    }

    #[inline]
    fn on_priority_prompt(&mut self, str: String) {
        if str.is_empty() {
            return self.todo_app.update_show_done_restriction();
        }
        let priority = str.parse();
        if let Ok(priority) = priority {
            self.todo_app
                .set_priority_restriction(priority, self.last_restriction.clone())
        }
    }

    #[inline]
    fn on_save_prompt(&mut self, str: HandlerParameter) {
        let lower = str.to_lowercase();
        if lower.starts_with('y') {
            let _ = self.todo_app.write();
        } else if lower.starts_with('c') {
            return;
        }
        let _ = self.quit();
    }

    #[inline]
    fn on_append_todo(&mut self, str: HandlerParameter) {
        self.todo_app.append(str);
    }

    #[inline]
    fn on_prepend_todo(&mut self, str: HandlerParameter) {
        self.todo_app.prepend(str);
    }

    #[inline]
    fn on_edit_todo(&mut self, str: HandlerParameter) {
        if !str.is_empty() {
            self.todo_app.todo_mut().unwrap().set_message(str);
        }
    }

    #[inline(always)]
    fn current_textarea_message(&self) -> String {
        self.textarea.lines()[0].clone()
    }

    #[inline]
    fn enable_text_editor(&mut self) -> io::Result<()> {
        let operation = self.editor()?;
        match operation {
            EditorOperation::Input => {
                if let Some(on_input) = self.on_input {
                    let message = self.current_textarea_message();
                    on_input(self, message);
                }
            }
            EditorOperation::Submit => {
                if let Some(on_submit) = self.on_submit {
                    let message = self.current_textarea_message();
                    on_submit(self, message);
                }
                self.turn_off_text_mode();
            }
            EditorOperation::Cancel => {
                self.turn_off_text_mode();
            }
            EditorOperation::Delete(before_delete) => {
                if let Some(on_delete) = self.on_delete {
                    let message = self.current_textarea_message();
                    on_delete(self, message, before_delete);
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub fn update_editor(&mut self) -> io::Result<Operation> {
        if self.args.enable_module {
            if event::poll(std::time::Duration::from_millis(
                self.module.update_time_ms(),
            ))? {
                self.enable_text_editor()?
            }
        } else {
            self.enable_text_editor()?
        }
        Ok(Operation::Nothing)
    }

    #[inline]
    fn editor(&mut self) -> io::Result<EditorOperation> {
        match crossterm::event::read()?.into() {
            Input {
                key: tui_textarea::Key::Esc,
                ..
            } => Ok(EditorOperation::Cancel),
            Input {
                key: tui_textarea::Key::Enter,
                ..
            } => Ok(EditorOperation::Submit),
            Input {
                key: tui_textarea::Key::Char('u'),
                ctrl: true,
                ..
            } => {
                let before_delete = self.current_textarea_message();
                self.textarea.delete_line_by_head();
                Ok(EditorOperation::Delete(before_delete))
            }
            input => {
                let is_backspace = input.key == tui_textarea::Key::Backspace;
                let before_delete = self.current_textarea_message();
                self.textarea.input(input);
                if is_backspace {
                    Ok(EditorOperation::Delete(before_delete))
                } else {
                    Ok(EditorOperation::Input)
                }
            }
        }
    }

    #[inline]
    pub fn update_return_operation(&mut self) -> io::Result<Operation> {
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
        if self.args.enable_module {
            if event::poll(std::time::Duration::from_millis(
                self.module.update_time_ms(),
            ))? {
                return self.read_keys();
            }
        } else {
            return self.read_keys();
        }
        Ok(Operation::Nothing)
    }

    #[inline]
    fn write(&mut self) -> io::Result<()> {
        self.todo_app.write()
    }

    #[inline]
    fn read_keys(&mut self) -> io::Result<Operation> {
        if let Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    Char('o') if key.modifiers == KeyModifiers::CONTROL => {
                        self.nnn_open();
                        return Ok(Operation::Restart);
                    }
                    Char('x') => self.todo_app.cut_todo(),
                    Char('d') => self.todo_app.toggle_current_daily(),
                    Char('W') => self.todo_app.toggle_current_weekly(),
                    Char('S') => self.schedule_prompt(),
                    Char('m') => self.reminder_prompt(),
                    Char('M') => self.todo_app.toggle_schedule(),
                    Char('!') => self.todo_app.toggle_show_done(),
                    Char('@') => self.priority_prompt(),
                    Char('y') => self.todo_app.yank_todo(),
                    Char('p') => self.todo_app.paste_todo(),
                    Char('i') => self.todo_app.increase_day_done(),
                    Char('I') => self.todo_app.decrease_day_done(),
                    Char('o') => {
                        self.nnn_append_todo();
                        return Ok(Operation::Restart);
                    }
                    Char('O') => {
                        self.nnn_output_todo();
                        return Ok(Operation::Restart);
                    }
                    KeyCode::Down | Char('j') => self.todo_app.increment(),
                    KeyCode::Up | Char('k') => self.todo_app.decrement(),
                    KeyCode::Right | Char('l') => self.todo_app.add_dependency_traverse_down(),
                    KeyCode::Enter => self.todo_app.traverse_down(),
                    KeyCode::Left | Char('h') => {
                        self.todo_app.traverse_up();
                    }
                    KeyCode::Home | Char('g') => self.todo_app.go_top(),
                    KeyCode::End | Char('G') => self.todo_app.go_bottom(),
                    Char('w') => self.write()?,
                    Char('J') => self.todo_app.decrease_current_priority(),
                    Char('K') => self.todo_app.increase_current_priority(),
                    Char(']') => self.show_right = !self.show_right,
                    Char('P') => self.args.enable_module = !self.args.enable_module,
                    Char('>') => {
                        self.todo_app.edit_or_add_note();
                        return Ok(Operation::Restart);
                    }
                    Char('t') => self.todo_app.add_dependency(),
                    Char('D') => {
                        self.todo_app.delete_todo();
                    }
                    Char('R') => self.todo_app.read(),
                    Char('T') => self.todo_app.remove_current_dependent(),
                    Char(' ') => self.todo_app.toggle_current_done(),
                    Char('n') => self.todo_app.search_next(),
                    Char('a') => self.prepend_prompt(),
                    Char('/') => self.search_prompt(),
                    Char('?') => self.tree_search_prompt(),
                    Char('A') => self.append_prompt(),
                    Char('e') | Char('E') => self.edit_prompt(key.code == Char('E')),
                    Char('r') if key.modifiers == KeyModifiers::CONTROL => self.edit_prompt(false),
                    Char('~') => self.todo_app.go_root(),
                    Char('q') => self.quit_save_prompt(),
                    Char('r') => {
                        self.todo_app.batch_editor_messages();
                        return Ok(Operation::Restart);
                    }
                    Char(c) if c.is_ascii_digit() => {
                        let priority = c.to_digit(10).unwrap();
                        self.todo_app.set_current_priority(priority as PriorityType);
                    }

                    Char('s') => self.module.on_s(),
                    Char('H') => self.module.on_capital_h(),
                    Char('c') => self.module.on_c(),
                    Char('C') => self.module.on_capital_c(),
                    Char('L') => self.module.on_capital_l(),
                    Char('f') => self.module.on_f(),
                    Char('+') | Char('=') => self.module.on_plus(),
                    Char('-') => self.module.on_minus(),
                    Char('.') => self.module.on_dot(),
                    Char(',') => self.module.on_comma(),
                    _ => {}
                }
            }
        }
        Ok(Operation::Nothing)
    }

    #[inline]
    fn get_dependency_width(&self, todo: Option<&Todo>) -> u16 {
        if let Some(todo) = todo {
            if !self.show_right || todo.dependency.is_none() || !self.todo_app.is_tree() {
                return 0;
            }
            40
        } else {
            0
        }
    }

    #[inline]
    fn render_module_widget(
        &self,
        frame: &mut Frame,
        direction: Direction,
        constraint1: Constraint,
        constraint2: Constraint,
    ) -> Rc<[Rect]> {
        let main_layout = Layout::default()
            .direction(direction)
            .constraints([constraint1, constraint2])
            .split(frame.size());
        frame.render_widget(self.module.get_widget(), main_layout[0]);
        main_layout
    }

    #[inline]
    fn highlight_string(&self) -> &str {
        self.args.highlight_string.as_str()
    }

    #[inline]
    fn render_dependency_widget(
        &self,
        frame: &mut Frame,
        todo: Option<&Todo>,
        dependency_layout: Rect,
    ) {
        if let Some(todo) = todo {
            if let Some(note) = todo.dependency.as_ref().and_then(|dep| dep.note()) {
                let note_widget = Paragraph::new(Text::styled(note, Style::default()))
                    .wrap(Wrap { trim: true })
                    .block(default_block("Todo note"));
                frame.render_widget(note_widget, dependency_layout);
            }
            if let Some(todo_list) = todo.dependency.as_ref().and_then(|dep| dep.todo_list()) {
                Self::render_todos_widget(
                    self.highlight_string(),
                    frame,
                    None,
                    dependency_layout,
                    self.todo_app.display_list(todo_list),
                    String::from("Todo dependencies"),
                    self.todo_app.index(),
                )
            }
        }
    }

    #[inline(always)]
    fn render_current_todos_widget(
        &mut self,
        frame: &mut Frame,
        list_state: &mut ListState,
        todo_layout: Rect,
    ) {
        let title = self.title();
        let display = if self.args.minimal_render {
            let first = self.todo_app.index();
            let last = self.todo_app.len().min(todo_layout.height as usize+first-2);
            self.todo_app.display_current_slice(first, last)
        } else {
            self.todo_app.display_current()
        };
        Self::render_todos_widget(
            self.highlight_string(),
            frame,
            Some(list_state),
            todo_layout,
            display,
            title,
            self.todo_app.index(),
        )
    }

    #[inline(always)]
    fn render_todos_widget(
        highlight_symbol: &str,
        frame: &mut Frame,
        list_state: Option<&mut ListState>,
        todo_layout: Rect,
        display_list: Vec<String>,
        title: String,
        index: usize,
    ) {
        match create_todo_widget(display_list, title, highlight_symbol) {
            TodoWidget::Paragraph(widget) => frame.render_widget(widget, todo_layout),
            TodoWidget::List(widget) => {
                if let Some(list_state) = list_state {
                    let widget = frame.render_stateful_widget(widget, todo_layout, list_state);
                    list_state.select(Some(index-list_state.offset()));
                    widget

                } else {
                    frame.render_widget(widget, todo_layout)
                }
            }
        }
    }

    #[inline]
    pub fn ui(&mut self, frame: &mut Frame, list_state: &mut ListState) {
        let todo = self.todo_app.todo();
        if self.args.minimal_render {
            list_state.select(Some(0));
        } else {
            list_state.select(Some(self.todo_app.index()));
        }

        let dependency_width = self.get_dependency_width(todo);

        let main_layout = if self.args.enable_module {
            self.render_module_widget(
                frame,
                Direction::Vertical,
                Constraint::Length(5),
                Constraint::Min(0),
            )
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0)])
                .split(frame.size())
        };

        let todo_app_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(100 - dependency_width),
                Constraint::Percentage(dependency_width),
            ])
            .split(main_layout[self.args.enable_module as usize]);

        let todo_and_textarea_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3 * self.text_mode as u16),
                Constraint::Min(0),
            ])
            .split(todo_app_layout[0]);
        self.render_dependency_widget(frame, todo, todo_app_layout[1]);

        frame.render_widget(self.textarea.widget(), todo_and_textarea_layout[0]);
        self.render_current_todos_widget(frame, list_state, todo_and_textarea_layout[1]);
    }
}
