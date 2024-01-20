// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::io::{self, stdout};
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
use tui::{startup, shutdown};
//}}}

fn main() -> io::Result<()> {
    startup()?;
    match run() {
        Err(err) => {
            shutdown()?;
            Err(err)
        }
        _ => {Ok(())}
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

        app.update(&mut terminal)?;
    }
}
