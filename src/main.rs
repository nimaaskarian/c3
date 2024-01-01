// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::{io::{self, stdout, Write}};
//}}}
// lib {{{
use ratatui::{prelude::*, widgets::*};
use crossterm::{
    event::{self, Event::Key, KeyCode::Char, KeyCode},
    ExecutableCommand,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
};
use tui_textarea::{Input, TextArea};
use tui_textarea;
//}}}
//mod{{{
pub mod todo_list;
pub mod fileio;
mod app;
use app::App;
use todo_list::todo::Todo;
use crate::todo_list::TodoArray;
//}}}

fn main() -> io::Result<()> {
    startup()?;
    let status = run();
    shutdown()?;
    status?;
    Ok(())
}

fn run() -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut list_state = ListState::default();
    let mut app = App::new();

    let mut textarea = TextArea::default();
    textarea.set_cursor_line_style(Style::default());

    loop {
        terminal.draw(|frame| {
            ui(frame, &app, &mut list_state, &textarea);
        })?;

        if !app.text_mode {
            if update(&mut app, &list_state, &mut textarea)? {
                terminal.clear()?;
                startup()?;
            }
        } else {
            match editor(&mut textarea)? {
                None => {},
                Some(should_add) => {
                    if should_add {
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
        input => {
            textarea.input(input) ;
            Ok(None)
        }
    }
}

fn update(app: &mut App, list_state: &ListState, textarea:&mut TextArea) -> io::Result<bool> {
    let size = app.current_list().undone.len();
    app.index = match size {
        0 => 0,
        _ => app.index.min(size-1),
    };

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
                    app.show_right = !app.show_right
                },
                Char('N') => {
                    if app.mut_todo().unwrap().edit_note().is_err() {
                        let _ = app.mut_todo().unwrap().add_note();
                    }
                    return Ok(true)
                },
                Char('t') => {
                    let _ = app.mut_todo().unwrap().add_dependency();
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
                Char('R') => {
                    app.read()
                },
                KeyCode::Delete => {
                    app.mut_todo().unwrap().remove_dependency();
                    app.mut_todo().unwrap().remove_note();
                }
                KeyCode::Enter => {
                    app.mut_todo().unwrap().toggle_done();
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
    Ok(false)
}

fn add_todo(str:String, app:&mut App) {
    app.mut_current_list().add(Todo::new(str, 0));
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
        return TodoWidget::Paragraph(Paragraph::new("No todo.").block(Block::default().title(title).borders(Borders::ALL)))
    }
    return TodoWidget::List(List::new(todo_array.display())
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true));

}

fn ui(frame: &mut Frame, app: &App, state:&mut ListState, textarea:&TextArea) {
    let todo = app.todo();

    state.select(Some(app.index));

    let note = match (todo, app.show_right) {
        (Some(todo), true)  => todo.get_note(),
        _ => String::new(),
    };

    let dependency_width = if let Some(todo) = todo {
        let should_show_right = (todo.has_dependency() || !todo.get_note().is_empty()) && app.show_right;
        40 * (should_show_right as u16)
    } else {
        0
    };

    let main_layout = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Percentage(100 - dependency_width),
            Constraint::Percentage(dependency_width),
        ]
    ).split(frame.size());

    let todo_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(3 * app.text_mode as u16),
            Constraint::Min(0),
        ]
    ).split(main_layout[0]);


    match create_todo_widget(&app.current_list().undone, app.title()) {
        TodoWidget::Paragraph(widget) => frame.render_widget(widget, todo_layout[1]),
        TodoWidget::List(widget) => frame.render_stateful_widget(widget, todo_layout[1], state),
    };

    frame.render_widget(textarea.widget(), todo_layout[0]);
    
    if todo.is_some() && app.show_right{
        let todo = todo.unwrap();
        if !todo.get_note().is_empty(){
            let note_widget = Paragraph::new(Text::styled(note, Style::default())).wrap(Wrap { trim: true }).block(Block::new().title("Todo note").borders(Borders::ALL));
            frame.render_widget(note_widget, main_layout[1]);
        } else
        if todo.has_dependency() {
            match create_todo_widget(&todo.dependencies.undone, String::from("Todo dependencies")) {
                TodoWidget::List(widget) =>frame.render_widget(widget, main_layout[1]),
                TodoWidget::Paragraph(widget) =>frame.render_widget(widget, main_layout[1]),
            }
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
