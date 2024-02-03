// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::io::{self};
use std::path::PathBuf;
//}}}
// lib {{{
use clap::Parser;
// }}}
//mod{{{
pub mod fileio;
pub(crate) mod todo_app;
pub(crate) mod cli_app;
pub(crate) mod tui_app;
use todo_app::App;
use fileio::get_todo_path;
//}}}


/// A tree-like todo application that makes you smile
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Show a tree like output (non interactive)
    #[arg(short='n', long)]
    no_tree: bool,

    /// List todos in non interactive mode
    #[arg(short='l', long)]
    non_interactive: bool,

    /// Show done todos too
    #[arg(short='d', long)]
    show_done: bool,

    /// Write contents of todo file in the stdout (non interactive)
    #[arg(short='s', long)]
    stdout: bool,

    /// Path to todo file (and notes sibling directory)
    #[arg(short='p', long,default_value=get_todo_path().unwrap().into_os_string())]
    todo_path: PathBuf,
}

impl Args {
    pub fn is_cli(&self) -> bool {
        self.stdout || self.non_interactive
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let is_cli = args.is_cli();
    let mut app = App::new(args);

    if is_cli {
        cli_app::run(&app)
    } else {
        tui_app::startup()?;
        match tui_app::run(&mut app) {
            Ok(_)=>{Ok(())}
            err => {
                tui_app::shutdown()?;
                err
            }
        }
    }
}
