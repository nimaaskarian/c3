use std::{io::{self, stdout, Write}, path::PathBuf, fs::File};
// use ratatui_textarea::TextArea;
use ratatui::{prelude::*, widgets::*};
use crossterm::{
    event::{self, Event::Key, KeyCode::Char, KeyCode},
    ExecutableCommand,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
};
use tui_textarea::{Input, TextArea};

pub mod todo_list;
pub mod fileio;

use tui_textarea;
use fileio::todo_path;
use todo_list::{TodoList, TodoArray};
use todo_list::todo::Todo;

struct App {
    todo_list: TodoList,
    should_quit: bool,
    index: usize,
    todo_path: PathBuf,
    changed:bool,
    show_note:bool,
    prior_indexes: Vec<usize>,
    text_mode: bool,
    on_submit: Option<fn(String, &mut App)->()>,
}

fn main() -> io::Result<()> {
    startup()?;
    let status = run();
    shutdown()?;
    status?;
    Ok(())
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
            length=>length-1,
            0=>0,
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

    pub fn todo(&self) -> Option<&Todo> {
        if self.todo_list.undone.len() == 0 {
            return None
        }
        if self.prior_indexes.len() == 0 {
            let mut index = self.index;
            if index >= self.todo_list.undone.len() {
                index = self.todo_list.undone.len()-1;
            }
            return Some(&self.todo_list.undone[index])
        }
        if self.current_list().undone.len() == 0 {
            return None
        }
        if self.current_list().undone.len() <= self.index {
            let last_index = self.current_list().undone.len()-1;
            return Some(&self.current_list().undone[last_index]);
        }
        if self.current_undone_empty() {
            return None
        }
        Some(&self.current_list().undone[self.index])
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

    pub fn mut_todo(&mut self) -> Option<&mut Todo> {
        self.changed = true;
        let index = self.index;
        if self.todo_list.undone.len() == 0 {
            return None
        }
        if self.prior_indexes.len() == 0 {
            return Some(&mut self.todo_list.undone[index])
        }
        if self.current_list().undone.len() <= self.index {
            let last_index = self.current_list().undone.len()-1;
            return Some(&mut self.mut_current_list().undone[last_index]);
        }
        Some(&mut self.mut_current_list().undone[index])
    }

    pub fn write(&mut self) -> io::Result<()> {
        self.changed = false;
        self.todo_list.write(&self.todo_path)?;
        Ok(())
    }

}

fn run() -> io::Result<()> {
    // ratatui terminal
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    // application state
    let mut list_state = ListState::default();
    let mut app = App::new();

    let mut textarea = TextArea::default();
    textarea.set_cursor_line_style(Style::default());
    // textarea.set_block(
    //     Block::default()
    //         .borders(Borders::ALL)
    //         .title("Add todo"),
    // );

    loop {
        terminal.draw(|frame| {
            ui(frame, &app, &mut list_state, &textarea);
        })?;

        if !app.text_mode {
            if update(&mut app, &list_state, &mut textarea)? {
                terminal.clear();
                startup();
            }
        } else {
            match editor(&mut textarea)? {
                None => {},
                Some(should_add) => {
                    if (should_add) {
                        let todo_message = textarea.lines()[0].clone();
                        app.on_submit.unwrap()(todo_message, &mut app);
                    }
                    textarea.delete_line_by_head();
                    app.text_mode = false;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn editor(textarea: &mut TextArea) -> io::Result<Option<bool>> {
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
            textarea.delete_line_by_head();
            Ok(None)
        },
        // Input {
        //     key: tui_textarea::Key::Enter, ..
        // } => Ok(true),
        input => {
            // TextArea::input returns if the input modified its text
            if textarea.input(input) {
                // is_valid = validate(&mut textarea);
            }
            Ok(None)
        }
}
}

fn update(app: &mut App, list_state: &ListState, textarea:&mut TextArea) -> io::Result<bool> {
    let size = app.current_list().undone.len() as isize - 1;

    if app.index > size as usize{
        app.index = size as usize;
    }

    if event::poll(std::time::Duration::from_millis(500))? {
        if let Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    Char('j') => app.increment(),
                    Char('k') => app.decrement(),
                    Char('g') => app.go_top(),
                    Char('G') => app.go_bottom(),
                    Char('H') => app.page_up(&list_state),
                    Char('w') => app.write()?,
                    Char('J') => {
                        app.mut_todo().unwrap().decrease_priority();
                        let index = app.index;
                        app.index = app.mut_current_list().undone.reorder(index);
                    },
                    Char('K') => {
                        app.mut_todo().unwrap().increase_priority();
                        let index = app.index;
                        app.index = app.mut_current_list().undone.reorder(index);
                    },
                    Char('n') => {
                        app.show_note = !app.show_note
                    },
                    Char('N') => {
                        if app.mut_todo().unwrap().edit_note().is_err() {
                            app.mut_todo().unwrap().add_note();
                        }
                        return Ok(true)
                    },
                    Char('t') => {
                        app.mut_todo().unwrap().add_dependency();
                    },
                    Char('h') => {
                        app.traverse_up()
                    },
                    Char('d') => {
                        let index = app.index;
                        app.mut_current_list().undone.remove(index);
                    },
                    Char('l') => {
                        app.traverse_down()
                    },
                    KeyCode::Enter => {
                        app.mut_todo().unwrap().toggle_done();
                        // app.mut_todo().toggle_done();
                        app.mut_todo().unwrap().dependencies.fix_undone();
                        app.mut_current_list().fix_undone();
                        if app.current_undone_empty() {
                            app.traverse_up();
                            app.mut_current_list().fix_undone();
                        }
                    }
                    Char('a') => {
                        app.set_text_mode(add_todo);
                        textarea.set_placeholder_text("Enter the todo message");
                        textarea.set_block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Add todo"),
                        );
                    }
                    Char('e') => {
                        app.set_text_mode(edit_todo);
                        let todo_message = app.todo().unwrap().message.as_str();
                        textarea.insert_str(todo_message);
                        textarea.set_block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Edit todo"),
                        );
                        textarea.set_placeholder_text(todo_message);
                    }
                    Char('q') => app.should_quit = true,
                    KeyCode::Char(c) if c.is_digit(10) => {
                        app.mut_todo().unwrap().set_priority(c.to_digit(10).unwrap() as i8);
                        let index = app.index;
                        app.index = app.mut_current_list().undone.reorder(index);
                    }
                    _ => {},
                }
            }
        }
    }
    Ok(false)
}

fn add_todo(str:String, app:&mut App) {
    app.mut_current_list().add(str, 0);
    app.mut_current_list().undone.sort();
}

fn edit_todo(str:String, app:&mut App) {
    if !str.is_empty() {
        app.mut_todo().unwrap().set_message(str);
    }
}

enum TodoWidget<'a> {
    List(ratatui::widgets::List<'a>),
    Paragraph(ratatui::widgets::Paragraph<'a>),
}

fn create_todo_widget(todo_array:&TodoArray, title:String) ->  TodoWidget {
    if todo_array.len() == 0 {
        return TodoWidget::Paragraph(Paragraph::new("No todo.").block(Block::default().title("Todos").borders(Borders::ALL)))
    }
    return TodoWidget::List(List::new(todo_array.display())
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true));

}

fn ui(frame: &mut Frame, app: &App, main_state:&mut ListState, textarea:&TextArea) {
    let todo = match app.todo() {
        Some(todo) => todo,
        None => {
            let main_layout = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(3 * app.text_mode as u16),
                Constraint::Min(0),
            ]
            ).split(frame.size());
            frame.render_widget(Paragraph::new("No todo.").block(Block::default().title("Todos").borders(Borders::ALL)) , main_layout[1]);
            frame.render_widget(textarea.widget(), main_layout[0]);
            return;
        }   
    };

    let note = if app.show_note {
        todo.note()
    } else {
        String::new()
    };

    let note_height = match note.lines().count() {
        0 => 0,
        count => count as u16 + 2,
    };

    let dependency_width = 40 * todo.has_dependency() as u16;

    let main_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Min(0),
            Constraint::Length(note_height),
        ]
    ).split(frame.size());

    let todo_layout = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Percentage(100 - dependency_width),
            Constraint::Percentage(dependency_width),
        ]
    ).split(main_layout[0]);

    let main_todo_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(3 * app.text_mode as u16),
            Constraint::Min(0),
        ]
    ).split(todo_layout[0]);

    main_state.select(Some(app.index));

    match create_todo_widget(&app.current_list().undone, app.title()) {
        TodoWidget::Paragraph(widget) => frame.render_widget(widget, main_todo_layout[1]),
        TodoWidget::List(widget) => frame.render_stateful_widget(widget, main_todo_layout[1], main_state),
    };

    frame.render_widget(textarea.widget(), main_todo_layout[0]);
    
    if app.show_note {
        let note_widget = Paragraph::new(note).block(Block::new().title("Todo note").borders(Borders::ALL));
        frame.render_widget(note_widget, main_layout[1]);
    }
    if todo.has_dependency() {
        match create_todo_widget(&todo.dependencies.undone, String::from("Todo dependencies")) {
            TodoWidget::List(widget) =>frame.render_widget(widget, todo_layout[1]),
            TodoWidget::Paragraph(widget) =>frame.render_widget(widget, todo_layout[1]),
        }
    } 
}

fn validate(textarea: &mut TextArea) -> bool {
    if let Err(err) = textarea.lines()[0].parse::<f64>() {
        textarea.set_style(Style::default().fg(Color::LightRed));
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("ERROR: {}", err)),
        );
        false
    } else {
        textarea.set_style(Style::default().fg(Color::LightGreen));
        textarea.set_block(Block::default().borders(Borders::ALL).title("OK"));
        true
    }
}

fn startup() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Ok(())
}

fn shutdown() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
