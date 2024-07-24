// vim:fileencoding=utf-8:foldmethod=marker
use clap::{Command, CommandFactory, Parser};
use std::io;
pub(crate) mod cli_app;
pub(crate) mod date;
pub(crate) mod fileio;
pub(crate) mod todo_app;
pub(crate) mod tui_app;
use crate::fileio::get_todo_path;
use clap_complete::{generate, Generator, Shell};
use std::path::PathBuf;
use todo_app::App;

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mode = args.mode();
    let mut app = App::new(args);

    match mode {
        AppMode::Completion(generator) => {
            print_completions(generator, &mut Args::command());
            Ok(())
        }
        AppMode::Cli => cli_app::run(&mut app),
        AppMode::Tui => match tui_app::run(&mut app) {
            Ok(_) => Ok(()),
            err => {
                tui_app::shutdown()?;
                err
            }
        },
    }
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct DisplayArgs {
    /// Show done todos too
    #[arg(short = 'd', long, default_value_t = false)]
    show_done: bool,

    /// String before done todos
    #[arg(long, default_value_t=String::from("[x] "))]
    done_string: String,

    /// String before undone todos
    #[arg(long, default_value_t=String::from("[ ] "))]
    undone_string: String,
}

/// A tree-like todo application that makes you smile
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Performance mode, don't read dependencies
    #[arg(short = 'n', long)]
    no_tree: bool,

    /// Search and select todo. Used for batch change operations
    #[arg(short = 'S', long)]
    search_and_select: Vec<String>,

    /// String behind highlighted todo in TUI mode
    #[arg(short='H', long, default_value_t=String::from(">>"))]
    highlight_string: String,

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

    #[command(flatten)]
    display_args: DisplayArgs,

    /// A todo message to append
    #[arg(short = 'a', long)]
    append_todo: Vec<String>,

    /// A todo message to prepend
    #[arg(short = 'A', long)]
    prepend_todo: Vec<String>,

    /// A todo file to append to current list
    #[arg(long)]
    append_file: Option<PathBuf>,

    /// Minimal tree with no tree graphics
    #[arg(short = 'M', long)]
    minimal_tree: bool,

    /// List todos (non interactive)
    #[arg(short = 'l', long)]
    list: bool,

    /// Enable TUI module at startup
    #[arg(short = 'm', long)]
    enable_module: bool,

    /// Write contents of todo file in the stdout (non interactive)
    #[arg(short = 's', long)]
    stdout: bool,

    /// Path to todo file (and notes sibling directory)
    #[arg(default_value=get_todo_path().unwrap().into_os_string())]
    todo_path: PathBuf,

    #[arg(short = 'c', long)]
    completion: Option<Shell>,
}

pub enum AppMode {
    Cli,
    Tui,
    Completion(Shell),
}

impl Args {
    pub fn mode(&self) -> AppMode {
        if let Some(generator) = self.completion {
            return AppMode::Completion(generator);
        }

        if self.stdout
            || self.minimal_tree
            || self.list
            || !self.search_and_select.is_empty()
            || !self.prepend_todo.is_empty()
            || !self.append_todo.is_empty()
            || self.append_file.is_some()
        {
            AppMode::Cli
        } else {
            AppMode::Tui
        }
    }
}
