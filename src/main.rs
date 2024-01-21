// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::io::{self};
//}}}
//mod{{{
pub mod todo_list;
pub mod fileio;
mod modules;
mod tui;
//}}}

fn main() -> io::Result<()> {
    tui::startup()?;
    match tui::run() {
        Ok(_)=>{Ok(())}
        err => {
            tui::shutdown()?;
            err
        }
    }
}
