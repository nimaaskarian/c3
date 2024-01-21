// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::io::{self};
//}}}
//mod{{{
pub mod todo_list;
pub mod fileio;
mod modules;
mod tui;
mod cli;
//}}}
fn main() -> io::Result<()> {
    let is_tui = std::env::args().count() == 1;

    if is_tui {
        tui::startup()?;
        match tui::run() {
            Ok(_)=>{Ok(())}
            err => {
                tui::shutdown()?;
                err
            }
        }
    } else {
        cli::run()
    }
}
