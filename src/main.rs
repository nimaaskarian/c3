// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::io::{self};
//}}}
// lib {{{
use ratatui::{prelude::*, widgets::*};
//}}}
//mod{{{
pub mod todo_list;
pub mod fileio;
mod app;
mod modules;
use modules::potato::Potato;
mod tui;
use app::App;
use tui::{startup, shutdown, redraw};
//}}}

fn main() -> io::Result<()> {
    startup()?;
    match run() {
        Ok(_)=>{Ok(())}
        err => {
            shutdown()?;
            err
        }
    }
}

#[inline]
fn run() -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut potato_module = Potato::new(None);
    let mut list_state = ListState::default();
    let mut app = App::new(&mut potato_module);

    loop {
        terminal.draw(|frame| {
            app.ui(frame, &mut list_state)
        })?;

        let should_redraw = app.update_return_should_redraw()?;
        if should_redraw {
            redraw(&mut terminal)?;
        }
    }
}
