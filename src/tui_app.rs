// vim:fileencoding=utf-8:foldmethod=marker
// imports {{{
use clap::Parser;
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
use std::{
    io::{self, BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    rc::Rc,
};
use tui_textarea::{CursorMove, Input, TextArea};
mod potato;
mod todo_buffer;
use todo_buffer::TodoBuffer;
use c3::{
    date,
    todo_app::{App, Restriction, Schedule, Todo},
};

use potato::Potato;
// }}}

#[derive(Debug)]
pub enum HandlerOperation {
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

#[derive(Default, PartialEq)]
enum Mode {
    #[default]
    Normal,
    Editing,
}

pub struct TuiApp<'a> {
    todo_buffer: TodoBuffer,
    last_restriction: Option<Restriction>,
    show_right: bool,
    mode: Mode,
    on_submit: Option<fn(&mut Self, String) -> ()>,
    on_delete: Option<fn(&mut Self, String, String) -> ()>,
    on_input: Option<fn(&mut Self, String) -> ()>,
    args: TuiArgs,
    potato_module: Potato,
    textarea: TextArea<'a>,
    todo_app: &'a mut App,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct TuiArgs {
    /// Alternative way of rendering, render minimum amount of todos
    #[arg(long)]
    minimal_render: bool,

    /// String behind highlighted todo in TUI mode
    #[arg(short='H', long, default_value_t=String::from(">>"))]
    highlight_string: String,

    /// Enable TUI module at startup
    #[arg(short = 'm', long)]
    enable_module: bool,
}

impl<'a> TuiApp<'a> {
    #[inline]
    pub fn new(app: &'a mut App, args: TuiArgs) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        TuiApp {
            todo_buffer: Default::default(),
            todo_app: app,
            args,
            textarea,
            potato_module: Default::default(),
            on_submit: None,
            on_input: None,
            on_delete: None,
            show_right: true,
            mode: Default::default(),
            last_restriction: None,
        }
    }

    #[inline]
    pub fn title(&mut self) -> String {
        let changed_str = if self.todo_app.current_list().changed {
            "*"
        } else {
            ""
        };
        let size = self
            .todo_app
            .current_list()
            .len(self.todo_app.get_restriction());
        let todo_string = format!("Todos ({size}){changed_str}");

        if let Some(parent) = self.todo_app.parent() {
            format!("{todo_string} {}", parent.message)
        } else {
            todo_string
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
        on_submit: fn(&mut Self, String) -> (),
        title: &'a str,
        placeholder: &str,
    ) {
        self.on_input = None;
        self.on_delete = None;
        self.on_submit = Some(on_submit);
        self.turn_on_text_mode(title, placeholder);
    }

    #[inline]
    pub fn set_responsive_text_mode(
        &mut self,
        on_input: fn(&mut Self, String) -> (),
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
        self.mode = Mode::Editing;
    }

    #[inline(always)]
    fn turn_off_text_mode(&mut self) {
        self.textarea.delete_line_by_head();
        self.textarea.delete_line_by_end();
        self.mode = Mode::Normal;
    }

    #[inline]
    pub fn search_prompt(&mut self) {
        const TITLE: &str = "Search todo";
        const PLACEHOLDER: &str = "Enter search query";
        self.last_restriction = Some(Rc::clone(self.todo_app.get_restriction()));
        self.on_submit = None;
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
    fn on_search(&mut self, query: String) {
        self.todo_app
            .set_restriction_with_last(Rc::new(move |todo| todo.matches(query.as_str())), self.last_restriction.clone())
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
    fn on_schedule(&mut self, str: String) {
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

    fn nnn_paths() -> Option<impl Iterator<Item = PathBuf>> {
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
            return Some(reader.lines().map(|x| PathBuf::from(x.unwrap_or_default())));
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
    fn on_reminder(&mut self, str: String) {
        if let Ok(date) = date::parse_user_input(&str) {
            if let Some(todo) = self.todo_app.todo_mut() {
                todo.schedule = Some(Schedule::new_reminder(date));
                self.todo_app.reorder_current();
            }
        }
    }

    #[inline]
    pub fn edit_prompt(&mut self, start: bool) {
        if let Some(message) = &self.todo_app.todo().map(|todo| todo.message.clone()) {
            self.set_text_mode(Self::on_edit_todo, "Edit todo", message);
            self.textarea.insert_str(message);
            if start {
                self.textarea.move_cursor(CursorMove::Head);
            }
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
        self.last_restriction = Some(self.todo_app.get_restriction().clone());
        self.set_text_mode(Self::on_priority_prompt, TITLE, PLACEHOLDER);
        self.set_responsive_text_mode(Self::on_priority_prompt, TITLE, PLACEHOLDER);
        self.on_delete = Some(Self::on_priority_delete);
    }

    #[inline]
    pub fn schedule_restriction_prompt(&mut self) {
        const TITLE: &str = "Limit schedule";
        const PLACEHOLDER: &str = "Enter schedule to show";
        self.last_restriction = Some(self.todo_app.get_restriction().clone());
        self.set_text_mode(Self::on_schedule_prompt, TITLE, PLACEHOLDER);
        self.set_responsive_text_mode(Self::on_schedule_prompt, TITLE, PLACEHOLDER);
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
        if self.todo_app.is_changed() || self.todo_app.current_list().changed {
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
                .set_restriction_with_last(Rc::new(move |todo| todo.priority() == priority), self.last_restriction.clone())
        }
    }

    #[inline]
    fn on_schedule_prompt(&mut self, str: String) {
        if str.is_empty() {
            return self.todo_app.update_show_done_restriction();
        }
        let schedule_day = str.parse();
        if let Ok(schedule_day) = schedule_day {
            self.todo_app
                .set_restriction_with_last(Rc::new(move |todo| {
                    todo.schedule.as_ref().map_or(0, |sch| if sch.is_reminder() {0} else {sch.days()}) == schedule_day
                }), self.last_restriction.clone())
        }
    }

    #[inline]
    fn on_save_prompt(&mut self, str: String) {
        let lower = str.to_lowercase();
        if lower.starts_with('y') {
            let _ = self.todo_app.write();
        } else if lower.starts_with('c') {
            return;
        }
        let _ = self.quit();
    }

    #[inline]
    fn on_append_todo(&mut self, str: String) {
        self.todo_app.append(str);
    }

    #[inline]
    fn on_prepend_todo(&mut self, str: String) {
        self.todo_app.prepend(str);
    }

    #[inline]
    fn on_edit_todo(&mut self, str: String) {
        if !str.is_empty() {
            if let Some(todo) = self.todo_app.todo_mut() {
                todo.message = str;
            }
        }
    }

    #[inline(always)]
    fn current_textarea_message(&self) -> String {
        self.textarea.lines()[0].clone()
    }

    #[inline]
    fn handle_text_input(&mut self) -> io::Result<HandlerOperation> {
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
        Ok(HandlerOperation::Nothing)
    }

    #[inline]
    fn editor(&mut self) -> io::Result<EditorOperation> {
        let event = event::read()?;
        if let Key(key) = event {
            match key.code {
                KeyCode::Esc => return Ok(EditorOperation::Cancel),
                KeyCode::Enter => return Ok(EditorOperation::Submit),
                Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    let before_delete = self.current_textarea_message();
                    self.textarea.delete_line_by_head();
                    return Ok(EditorOperation::Delete(before_delete))
                }
                KeyCode::Backspace => {
                    let before_delete = self.current_textarea_message();
                    self.textarea.delete_char();
                    return Ok(EditorOperation::Delete(before_delete))
                }
                _ => {}
            }
        }
        let input:Input = event.into();
        self.textarea.input(input);
        Ok(EditorOperation::Input)
    }

    #[inline]
    pub fn handle_key_and_return_operation(&mut self) -> io::Result<HandlerOperation> {
        let input_handler = match self.mode {
            Mode::Editing => Self::handle_text_input,
            Mode::Normal => Self::handle_normal_input,
        };
        if self.args.enable_module {
            if event::poll(std::time::Duration::from_millis(
                self.potato_module.update_time_ms(),
            ))? {
                return input_handler(self);
            }
        } else {
            return input_handler(self);
        }
        Ok(HandlerOperation::Nothing)
    }

    #[inline]
    fn write(&mut self) -> io::Result<()> {
        self.todo_app.write()
    }

    #[inline]
    fn handle_normal_input(&mut self) -> io::Result<HandlerOperation> {
        let event = event::read()?;
        if let Key(key) = event {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    Char('o') if key.modifiers == KeyModifiers::CONTROL => {
                        self.nnn_open();
                        return Ok(HandlerOperation::Restart);
                    }
                    Char('x') => {
                        self.todo_app.remove_todo();
                        if let Some(todo) = self.todo_app.removed_todos.pop() {
                            self.todo_buffer.yank(todo);
                        }
                    },
                    Char('d') => self.todo_app.toggle_current_daily(),
                    Char('W') => self.todo_app.toggle_current_weekly(),
                    Char('S') => self.schedule_prompt(),
                    Char('m') => self.reminder_prompt(),
                    Char('M') => {
                        if let Some(todo) = self.todo_app.todo_mut() {
                            todo.toggle_schedule();
                        }
                    }
                    Char('!') => self.todo_app.toggle_show_done(),
                    Char('@') => self.priority_prompt(),
                    Char('%') => self.schedule_restriction_prompt(),
                    Char('y') => {
                        let todo = self.todo_app.todo().map(|t| t.clone());
                        self.todo_buffer.yank(todo);
                    }
                    Char('p') => {
                        if let Some(todo) = self.todo_buffer.get() {
                            let list = self.todo_app.current_list_mut();
                            list.push(todo);
                            self.todo_app.index = list.reorder_last();
                        }
                    }
                    Char('i') => self.todo_app.increase_day_by(1),
                    Char('I') => self.todo_app.increase_day_by(-1),
                    Char('o') => {
                        self.nnn_append_todo();
                        return Ok(HandlerOperation::Restart);
                    }
                    Char('O') => {
                        self.nnn_output_todo();
                        return Ok(HandlerOperation::Restart);
                    }
                    KeyCode::Down | Char('j') => self.todo_app.go_down(),
                    KeyCode::Up | Char('k') => self.todo_app.go_up(),
                    KeyCode::Right | Char('l') => self.todo_app.add_dependency_traverse_down(),
                    KeyCode::Enter => self.todo_app.traverse_down(),
                    KeyCode::Left | Char('h') => {
                        self.todo_app.traverse_up();
                    }
                    KeyCode::Home | Char('g') => {
                        self.todo_app.index = 0;
                    }
                    KeyCode::End | Char('G') => self.todo_app.index = self.todo_app.bottom(),
                    Char('w') => self.write()?,
                    Char('J') => self.todo_app.move_current_down(),
                    Char('K') => self.todo_app.move_current_up(),
                    Char(']') => self.show_right = !self.show_right,
                    Char('P') => self.args.enable_module = !self.args.enable_module,
                    Char('>') => {
                        self.todo_app.edit_or_add_note();
                        return Ok(HandlerOperation::Restart);
                    }
                    Char('t') => self.todo_app.add_dependency(),
                    Char('D') => {
                        self.todo_app.remove_todo();
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
                        return Ok(HandlerOperation::Restart);
                    }
                    Char(c) if c.is_ascii_digit() => {
                        let priority = c.to_digit(10).unwrap();
                        self.todo_app.set_current_priority(priority as u8);
                    }

                    Char('s') => self.potato_module.skip(),
                    Char('H') => self.potato_module.increase_timer(),
                    Char('c') => self.potato_module.toggle_pause(),
                    Char('C') => self.potato_module.quit(),
                    Char('L') => self.potato_module.decrease_timer(),
                    Char('f') => self.potato_module.restart(),
                    Char('+') | Char('=') => self.potato_module.increase_pomodoro(),
                    Char('-') => self.potato_module.decrease_pomodoro(),
                    Char('.') => self.potato_module.next(),
                    Char(',') => self.potato_module.prev(),
                    _ => {}
                }
            }
        }
        Ok(HandlerOperation::Nothing)
    }

    #[inline]
    fn is_dependency_enabled(&self, todo: Option<&Todo>) -> bool {
        todo.map_or(false, |todo| {
            self.show_right && todo.dependency.is_some() && self.todo_app.is_tree()
        })
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
        frame.render_widget(self.potato_module.get_widget(), main_layout[0]);
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
                    self.todo_app.display_a_slice(todo_list, 0, dependency_layout.height as usize - 2),
                    String::from("Todo dependencies"),
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
            let last = self
                .todo_app
                .current_list()
                .len(self.todo_app.get_restriction())
                .min(todo_layout.height as usize + first - 2);
            self.todo_app.display_a_slice(self.todo_app.current_list(), first, last)
        } else {
            self.todo_app.display_current_list()
        };
        Self::render_todos_widget(
            self.highlight_string(),
            frame,
            Some(list_state),
            todo_layout,
            display,
            title,
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
    ) {
        match create_todo_widget(display_list, title, highlight_symbol) {
            TodoWidget::Paragraph(widget) => frame.render_widget(widget, todo_layout),
            TodoWidget::List(widget) => {
                if let Some(list_state) = list_state {
                    frame.render_stateful_widget(widget, todo_layout, list_state)
                } else {
                    frame.render_widget(widget, todo_layout)
                }
            }
        }
    }

    #[inline]
    pub fn ui(&mut self, frame: &mut Frame, list_state: &mut ListState) {
        let todo = self.todo_app.todo();
        if !self.args.minimal_render {
            list_state.select(Some(self.todo_app.index()));
        }

        let dependency_enabled = self.is_dependency_enabled(todo);
        let dependency_width = if dependency_enabled { 40 } else { 0 };

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
        let is_editing = self.mode == Mode::Editing;

        let todo_and_textarea_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3 * is_editing as u16),
                Constraint::Min(0),
            ])
            .split(todo_app_layout[0]);
        if dependency_enabled {
            self.render_dependency_widget(frame, todo, todo_app_layout[1]);
        }

        if is_editing {
            frame.render_widget(self.textarea.widget(), todo_and_textarea_layout[0]);
        }
        self.render_current_todos_widget(frame, list_state, todo_and_textarea_layout[1]);
    }
}

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
    io::stdout()
        .execute(LeaveAlternateScreen)?
        .execute(crossterm::cursor::Show)?;
    Ok(())
}

pub fn startup() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout()
        .execute(EnterAlternateScreen)?
        .execute(crossterm::cursor::Hide)?;
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

    let mut list_state = ListState::default().with_selected(Some(0));
    let mut app = TuiApp::new(app, args);

    loop {
        terminal.draw(|frame| app.ui(frame, &mut list_state))?;

        let operation = app.handle_key_and_return_operation()?;
        match operation {
            HandlerOperation::Restart => restart(&mut terminal)?,
            HandlerOperation::Nothing => {}
        }
    }
}
