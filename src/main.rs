// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::io::{self, stdout};
//}}}
// lib {{{
use ratatui::{prelude::*, widgets::*};
use crossterm::{
    event::{self, Event::Key, KeyCode::Char, KeyCode},
    ExecutableCommand,
    terminal::{enable_raw_mode, EnterAlternateScreen}
};
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
//}}}

fn main() -> io::Result<()> {
    startup()?;
    run()?;
    Ok(())
}

fn startup() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Ok(())
}

#[inline]
fn run() -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut list_state = ListState::default();
    let mut potato_module = Potato::new(None);
    let mut app = App::new(&mut potato_module);

    loop {
        terminal.draw(|frame| {
            ui(frame, &app, &mut list_state);
        })?;

        if !app.text_mode {
            if update(&mut app)? {
                terminal.clear()?;
                startup()?;
            }
        } else {
            app.update_text_editor()?;
        }
    }
}

fn update(app: &mut App) -> io::Result<bool> {
    let size = app.len();
    app.index = match size {
        0 => 0,
        _ => app.index.min(size-1),
    };

    if app.module_enabled {
        if event::poll(std::time::Duration::from_millis(500))? {
            return read_keys(app);
        }
    } else {
        return read_keys(app);
    }
    Ok(false)
}

#[inline]
fn read_keys(app: &mut App)  -> io::Result<bool> {
    if let Key(key) = event::read()? {
        if key.kind == event::KeyEventKind::Press {
            match key.code {
                Char('d') | Char('x') => app.cut_todo(),
                Char('!') => app.toggle_include_done(),
                Char('y') => app.yank_todo(),
                Char('p') => app.paste_todo(),
                Char('j') => app.increment(),
                Char('k') => app.decrement(),
                Char('g') => app.go_top(),
                Char('G') => app.go_bottom(),
                Char('w') => app.write()?,
                Char('J') => app.increase_current_priority(),
                Char('K') => app.decrease_current_priority(),
                Char(']') => {
                    app.show_right = !app.show_right
                },
                Char('P') => {
                    app.module_enabled = !app.module_enabled
                },
                Char('>') => {
                    app.edit_or_add_note();
                    return Ok(true)
                },
                Char('t') => app.add_dependency(),
                Char('h') => app.traverse_up(),
                Char('D') => app.delete_todo(),
                Char('l') => app.add_dependency_traverse_down(),
                Char('R') => app.read(),
                Char('T') => app.remove_current_dependent(),
                KeyCode::Enter => app.toggle_current_done(),
                Char('n') => app.search_next(),
                Char('N') => app.search_prev(),
                Char('a') => app.prepend_prompt(),
                Char('/') => app.search_prompt(),
                Char('A') => app.append_prompt(),
                Char('E') | Char('e') => app.edit_prompt(key.code == Char('E')),
                Char('q') => app.quit_save_prompt(),
                Char(' ') => app.module.on_space(),
                Char('s') => app.module.on_s(),
                Char('H') => app.module.on_capital_h(),
                Char('L') => app.module.on_capital_l(),
                Char('r') => app.module.on_r(),
                Char('+') | Char('=') => app.module.on_plus(),
                Char('-') => app.module.on_minus(),
                Char('.') => app.module.on_dot(),
                Char(',') => app.module.on_comma(),
                Char(c) if c.is_digit(10) => app.set_current_priority(c.to_digit(10).unwrap() as i8),
                _ => {},
            }
        }
    }
    Ok(false)
}


fn ui(frame: &mut Frame, app: &App, state:&mut ListState) {
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

    frame.render_widget(app.textarea.widget(), todo_layout[0]);
    
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

