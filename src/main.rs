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
    /// Performance mode, don't read dependencies
    #[arg(short='n', long)]
    no_tree: bool,

    /// Search and select todo. Used for batch change operations
    #[arg(short='S', long)]
    search_and_select: Vec<String>,

    /// Set selected todo priority
    #[arg(long)]
    set_selected_priority: Option<u8>,

    /// Set selected todo message
    #[arg(long)]
    set_selected_message: Option<String>,

    /// Delete selected todos
    #[arg(long)]
    delete_selected: bool,

    /// Done selected todos
    #[arg(long)]
    done_selected: bool,

    /// Append todo
    #[arg(short='a', long)]
    append_todo: Vec<String>,

    /// Prepend todo
    #[arg(short='A', long)]
    prepend_todo: Vec<String>,

    /// Minimal tree with no tree graphics
    #[arg(short='M', long)]
    minimal_tree: bool,

    /// List todos (non interactive)
    #[arg(short='l', long)]
    list: bool,

    /// Show done todos too
    #[arg(short='d', long)]
    show_done: bool,

    /// Enable TUI module at startup
    #[arg(short='m', long)]
    enable_module: bool,

    /// Write contents of todo file in the stdout (non interactive)
    #[arg(short='s', long)]
    stdout: bool,

    /// Path to todo file (and notes sibling directory)
    #[arg(short='p', long,default_value=get_todo_path().unwrap().into_os_string())]
    todo_path: PathBuf,
}

impl Args {
    pub fn is_cli(&self) -> bool {
        self.stdout || self.list || !self.search_and_select.is_empty() || !self.prepend_todo.is_empty() || !self.append_todo.is_empty()
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let is_cli = args.is_cli();
    let mut app = App::new(args);

    if is_cli {
        cli_app::run(&mut app)
    } else {
        match tui_app::run(&mut app) {
            Ok(_)=>{Ok(())}
            err => {
                tui_app::shutdown()?;
                err
            }
        }
    }
}
