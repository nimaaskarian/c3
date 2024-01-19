// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::{io::{self, stdout}, process::Command};
//}}}
// lib {{{
use ratatui::{prelude::*, widgets::*};
use crossterm::{
    event::{self, Event::Key, KeyCode::Char, KeyCode},
    ExecutableCommand,
    terminal::{enable_raw_mode, EnterAlternateScreen}
};
use tui_textarea::{Input, TextArea, CursorMove};
use tui_textarea;
//}}}
//mod{{{
pub mod todo_list;
pub mod fileio;
mod app;
mod modules;
use modules::potato::Potato;
mod tui;
use tui::{default_block, create_todo_widget, TodoWidget};
use app::App;
use todo_list::todo::Todo;
//}}}

fn main() -> io::Result<()> {
    startup()?;
    run()?;
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
    let mut potato_module = Potato::new(None);
    let mut app = App::new(&mut potato_module);

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
            if app.module_enabled {
                if event::poll(std::time::Duration::from_millis(500))? {
                    enable_text_editor(&mut app, &mut textarea)?;
                }
            } else {
                enable_text_editor(&mut app, &mut textarea)?;
            }
        }
    }
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
                    if !app.is_todos_empty() {
                        let index = app.index;
                        let todo = app.mut_current_list().remove(index);
                        let todo_string:String = (&todo).into();
                        if let Some(clipboard) = &mut app.clipboard {
                            let _ = clipboard.set_text(todo_string);
                        }
                    }
                }
                Char('!') => {
                    app.toggle_include_done();
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
                    if let Some(todo) = app.mut_todo() {
                        todo.decrease_priority();
                        let index = app.index;
                        app.index = app.mut_current_list().reorder(index);
                    }
                },
                Char('K') => {
                    if let Some(todo) = app.mut_todo() {
                        todo.increase_priority();
                        let index = app.index;
                        app.index = app.mut_current_list().reorder(index);
                    }
                },
                Char(']') => {
                    app.show_right = !app.show_right
                },
                Char('P') => {
                    app.module_enabled = !app.module_enabled
                },
                Char('>') => {
                    if let Some(todo) = app.mut_todo() {
                        if todo.edit_note().is_err() {
                            todo.add_note();
                        }
                    }
                },
                Char('t') => {
                    if let Some(todo) = app.mut_todo() {
                        todo.add_dependency();
                    }
                },
                Char('h') => {
                    app.traverse_up()
                },
                Char('D') => {
                    if !app.is_todos_empty() {
                        let index = app.index;
                        app.mut_current_list().undone.remove(index);
                    }
                },
                Char('l') => {
                    if let Some(todo) = app.todo() {
                        if !todo.has_dependency() && todo.note_empty() {
                            app.mut_todo().unwrap().add_dependency();
                        }
                    }
                    app.traverse_down()
                },
                Char('R') => {
                    app.read()
                },
                Char('T') => {
                    if let Some(todo) = app.mut_todo() {
                        todo.remove_note();
                        todo.remove_dependency();
                    }
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
                Char('/') => {
                    app.set_text_mode(search_todo);
                    textarea.set_placeholder_text("Enter query");
                    textarea.set_block(
                        default_block("Search todo")
                    );
                }
                Char('n') => {
                    app.search_next();
                }
                Char('N') => {
                    app.search_prev();
                }
                Char('A') => {
                    app.set_text_mode(add_todo_priority_one);
                    textarea.set_placeholder_text("Enter the todo message");
                    textarea.set_block(
                        default_block("Add todo at first")
                    );
                }
                Char(' ') => {
                    app.module.on_space()
                }
                Char('s') => {
                    app.module.on_s()
                }
                Char('H') => {
                    app.module.on_capital_h()
                }
                Char('L') => {
                    app.module.on_capital_l()
                }
                Char('r') => {
                    app.module.on_r()
                }
                Char('+') | Char('=') => {
                    app.module.on_plus()
                }
                Char('-') => {
                    app.module.on_minus()
                }
                Char('.') => {
                    app.module.on_dot()
                }
                Char(',') => {
                    app.module.on_comma()
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
                Char('q') => {
                    if app.changed {
                        app.set_text_mode(save_prompt);
                        textarea.set_placeholder_text("N/y/c");
                        textarea.set_block(
                            default_block("You have done changes. You wanna save em? [n: no, y: yes, c: cancel]")
                        );
                    } else {
                        app.quit();
                    }
                }
                Char(c) if c.is_digit(10) => {
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

    if app.module_enabled {
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

fn search_todo(str:String, app:&mut App) {
    app.search(Some(str));
    app.search_next_index();
}

fn add_todo_priority_one(str:String, app:&mut App) {
    app.mut_current_list().prepend(Todo::new(str, 1));
    app.index = 0;
}


fn edit_todo(str:String, app:&mut App) {
    if !str.is_empty() {
        app.mut_todo().unwrap().set_message(str);
    }
}

fn save_prompt(str:String, app:&mut App) {
    let lower = str.to_lowercase();
    if lower.starts_with("y") {
        app.write();
    } else if lower.starts_with("c") {
        return;
    }
    app.quit();
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
    let main_layout = if app.module_enabled {
         let main_layout = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(5),
                Constraint::Min(0),
            ]
        ).split(frame.size());
        frame.render_widget(app.module.get_widget(), main_layout[0]);
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
    ).split(main_layout[app.module_enabled as usize]);

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

