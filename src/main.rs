// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::{io::{self, stdout, Write}, process::Command};
//}}}
// lib {{{
use ratatui::{prelude::*, widgets::*};
use crossterm::{
    event::{self, Event::Key, KeyCode::Char, KeyCode},
    ExecutableCommand,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
};
use tui_textarea::{Input, TextArea, CursorMove};
use tui_textarea;
//}}}
//mod{{{
pub mod todo_list;
pub mod fileio;
mod app;
use app::App;
use todo_list::{todo::Todo, TodoList};
use crate::todo_list::TodoArray;
//}}}

fn main() -> io::Result<()> {
    startup()?;
    let status = run();
    shutdown()?;
    status?;
    Ok(())
}

#[inline]
fn enable_text_editor(app:&mut App , textarea: &mut TextArea) -> io::Result<()>{
    match editor(textarea)? {
        None => {},
        Some(should_add) => {
            if should_add {
                let todo_message = textarea.lines()[0].clone();
                app.on_submit.unwrap()(todo_message, app);
            }
            textarea.delete_line_by_head();
            textarea.delete_line_by_end();
            app.text_mode = false;
        }
    }
    Ok(())
}

#[inline]
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
            if update(&mut app, &mut textarea)? {
                terminal.clear()?;
                startup()?;
            }
        } else {
            if app.potato {
                if event::poll(std::time::Duration::from_millis(500))? {
                    enable_text_editor(&mut app, &mut textarea)?;
                }
            } else {
                enable_text_editor(&mut app, &mut textarea)?;
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

#[inline]
fn read_keys(app: &mut App, textarea:&mut TextArea)  -> io::Result<bool> {
    if let Key(key) = event::read()? {
        if key.kind == event::KeyEventKind::Press {
            match key.code {
                Char('d') | Char('x') => {
                    let index = app.index;
                    let todo = app.mut_current_list().undone.remove(index);
                    let todo_string:String = (&todo).into();
                    if let Some(clipboard) = &mut app.clipboard {
                        let _ = clipboard.set_text(todo_string);
                    }
                }
                Char('!') => {
                    app.include_done = !app.include_done;
                }
                Char('y') => {
                    let todo_string:String = app.todo().unwrap().into();
                    if let Some(clipboard) = &mut app.clipboard {
                        let _ = clipboard.set_text(todo_string);
                    }
                }
                Char('p') => {
                    if let Some(clipboard) = &mut app.clipboard {
                        if let Ok(text) = clipboard.get_text() {
                            match Todo::try_from(text) {
                                Ok(todo) => {
                                    app.mut_current_list().add(todo);
                                    app.mut_current_list().undone.sort();
                                },
                                _ => {},
                            };
                        }
                    }
                }
                Char('j') => app.increment(),
                Char('k') => app.decrement(),
                Char('g') => app.go_top(),
                Char('G') => app.go_bottom(),
                Char('w') => app.write()?,
                Char('J') => {
                    app.mut_todo().unwrap().decrease_priority();
                    let index = app.index;
                    app.index = app.mut_current_list().reorder(index);
                },
                Char('K') => {
                    app.mut_todo().unwrap().increase_priority();
                    let index = app.index;
                    app.index = app.mut_current_list().reorder(index);
                },
                Char('n') => {
                    app.show_right = !app.show_right
                },
                Char('P') => {
                    app.potato = !app.potato
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
                Char('D') => {
                    let index = app.index;
                    app.mut_current_list().undone.remove(index);
                },
                Char('l') => {
                    app.traverse_down()
                },
                Char('R') => {
                    app.read()
                },
                Char('T') => {
                    app.mut_todo().unwrap().remove_dependency();
                    app.mut_todo().unwrap().remove_note();
                }
                KeyCode::Enter => {
                    app.toggle_current_done();
                }
                Char('a') => {
                    app.set_text_mode(add_todo);
                    textarea.set_placeholder_text("Enter the todo message");
                    textarea.set_block(
                        default_block("Add todo")
                    );
                }
                // Char('/') => {
                //     app.set_text_mode(search_todo);
                //     textarea.set_placeholder_text("Enter query");
                //     textarea.set_block(
                //         default_block("Search todo")
                //     );
                // }
                Char('A') => {
                    app.set_text_mode(add_todo_priority_one);
                    textarea.set_placeholder_text("Enter the todo message");
                    textarea.set_block(
                        default_block("Add todo at first")
                    );
                }
                Char(' ') => {
                    Command::new("potctl").args(["-t0"]).status();
                }
                Char('s') => {
                    Command::new("potctl").args(["-s0"]).status();
                }
                Char('H') => {
                    Command::new("potctl").args(["-i0"]).status();
                }
                Char('L') => {
                    Command::new("potctl").args(["-d0"]).status();
                }
                Char('r') => {
                    Command::new("potctl").args(["-r0"]).status();
                }
                Char('+') | Char('=') => {
                    Command::new("potctl").args(["-I0"]).status();
                }
                Char('-') => {
                    Command::new("potctl").args(["-D0"]).status();
                }
                Char('E') | Char('e') => {
                    app.set_text_mode(edit_todo);
                    let todo_message = app.todo().unwrap().message.as_str();
                    textarea.insert_str(todo_message);
                    textarea.set_block(
                        default_block("Edit todo")
                    );
                    textarea.set_placeholder_text(todo_message);
                    if key.code == Char('E') {
                        textarea.move_cursor(CursorMove::Head);
                    }
                }
                Char('q') => app.should_quit = true,
                KeyCode::Char(c) if c.is_digit(10) => {
                    app.mut_todo().unwrap().set_priority(c.to_digit(10).unwrap() as i8);
                    let index = app.index;
                    app.index = app.mut_current_list().reorder(index);
                }
                _ => {},
            }
        }
    }
    Ok(false)
}

fn update(app: &mut App, textarea:&mut TextArea) -> io::Result<bool> {
    let size = app.len();
    app.index = match size {
        0 => 0,
        _ => app.index.min(size-1),
    };

    if app.potato {
        if event::poll(std::time::Duration::from_millis(500))? {
            return read_keys(app, textarea);
        }
    } else {
        return read_keys(app, textarea);
    }
    Ok(false)
}

fn add_todo(str:String, app:&mut App) {
    app.mut_current_list().add(Todo::new(str, 0));
    app.index = app.current_list().undone.len()-1;
}

// fn search_todo(str:String, app:&mut App) {
//     app.search(str);
// }

fn add_todo_priority_one(str:String, app:&mut App) {
    app.mut_current_list().prepend(Todo::new(str, 1));
    app.index = 0;
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

fn default_block<'a, T>(title: T) -> Block<'a> 
where
    T: Into<Line<'a>>,
{
    Block::default().title(title).borders(Borders::ALL)
}

fn create_todo_widget(display_list:&Vec<String>, title:String) ->  TodoWidget {
    if display_list.len() == 0 {
        return TodoWidget::Paragraph(Paragraph::new("No todo.").block(default_block(title)))
    }
    return TodoWidget::List(List::new((*display_list).clone())
        .block(default_block(title))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true));

}

fn get_potato_widget<'a>() -> Paragraph<'a> {
    let time_str = Command::new("potctl").args(["+%m\n%t\n%p", "-10"]).output().unwrap();
    let time_str = String::from_utf8(time_str.stdout).unwrap();

    Paragraph::new(time_str).block(default_block("Potato"))
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
        Direction::Vertical,
        [
            Constraint::Length(5* app.potato as u16),
            Constraint::Min(0),
        ]
    ).split(frame.size());
    frame.render_widget(get_potato_widget(), main_layout[0]);

    let todos_layout = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Percentage(100 - dependency_width),
            Constraint::Percentage(dependency_width),
        ]
    ).split(main_layout[1]);

    let todo_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(3 * app.text_mode as u16),
            Constraint::Min(0),
        ]
    ).split(todos_layout[0]);


    match create_todo_widget(&app.display(), app.title()) {
        TodoWidget::Paragraph(widget) => frame.render_widget(widget, todo_layout[1]),
        TodoWidget::List(widget) => frame.render_stateful_widget(widget, todo_layout[1], state),
    };

    frame.render_widget(textarea.widget(), todo_layout[0]);
    
    if todo.is_some() && app.show_right{
        let todo = todo.unwrap();
        if !todo.get_note().is_empty(){
            let note_widget = Paragraph::new(Text::styled(note, Style::default())).wrap(Wrap { trim: true }).block(default_block("Todo note"));
            frame.render_widget(note_widget, todos_layout[1]);
        } else
        if todo.has_dependency() {
            match create_todo_widget(&todo.dependencies.display(app.include_done), String::from("Todo dependencies")) {
                TodoWidget::List(widget) =>frame.render_widget(widget, todos_layout[1]),
                TodoWidget::Paragraph(widget) =>frame.render_widget(widget, todos_layout[1]),
            }
        } 
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
