// vim:fileencoding=utf-8:foldmethod=marker
// standard {{{
use std::io::{self};
//}}}
//mod{{{
pub mod todo_list;
pub mod fileio;
mod tui;
mod cli;
//}}}

use clap::Parser;
use std::path::PathBuf;
/// A tree-like todo application that makes you smile
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Show a tree like output (non interactive)
    #[arg(short, long)]
    tree: bool,

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
    #[arg(short='p', long)]
    todo_path: Option<PathBuf>,
}

impl Args {
    pub fn is_cli(&self) -> bool {
        self.stdout | self.non_interactive | self.tree
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    if args.is_cli() {
        cli::run(args)
    } else {
        tui::startup()?;
        match tui::run(args) {
            Ok(_)=>{Ok(())}
            err => {
                tui::shutdown()?;
                err
            }
        }
    }
}
