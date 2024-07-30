// vim:fileencoding=utf-8:foldmethod=marker
use clap::{Parser, ValueEnum};
use clap_complete::Shell;
use std::io;
pub(crate) mod cli_app;
pub(crate) mod date;
pub(crate) mod fileio;
pub(crate) mod todo_app;
pub(crate) mod tui_app;
use crate::fileio::get_todo_path;
use std::path::PathBuf;
use todo_app::App;

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mut app = App::new(args.app_args);

    if cli_app::run(&mut app, args.cli_args).is_err() {
        let output = tui_app::run(&mut app, args.tui_args);
        {
            tui_app::shutdown()?;
            output
        }
    } else {
        Ok(())
    }
}

/// A tree-like todo application that makes you smile
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(flatten)]
    app_args: AppArgs,

    #[command(flatten)]
    cli_args: CliArgs,

    #[command(flatten)]
    tui_args: TuiArgs,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum DoOnSelected {
    Delete,
    Done,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// Search and select todo. Used for batch change operations
    #[arg(short = 'S', long)]
    search_and_select: Vec<String>,

    #[arg(long)]
    do_on_selected: Option<DoOnSelected>,

    #[arg(short = 'b', long, default_value_t = false)]
    batch_edit: bool,

    /// A todo message to append
    #[arg(short = 'a', long)]
    append_todo: Vec<String>,

    /// A todo message to prepend
    #[arg(short = 'A', long)]
    prepend_todo: Vec<String>,

    /// A todo file to append to current list
    #[arg(long)]
    append_file: Option<PathBuf>,

    /// A todo file to output to
    #[arg(short = 'o', long)]
    output_file: Option<PathBuf>,

    #[arg(short = 'p', long, default_value_t = false)]
    print_path: bool,

    /// Minimal tree with no tree graphics
    #[arg(short = 'M', long)]
    minimal_tree: bool,

    /// List todos (non interactive)
    #[arg(short = 'l', long)]
    list: bool,

    /// Write contents of todo file in the stdout (non interactive)
    #[arg(short = 's', long)]
    stdout: bool,

    /// Generate completion for a certain shell
    #[arg(short = 'c', long)]
    completion: Option<Shell>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct TuiArgs {
    /// Alternative way of rendering, render minimum amount of todos
    #[arg(long)]
    minimal_render: bool,

    /// String behind highlighted todo in TUI mode
    #[arg(short='H', long, default_value_t=String::from(">>"))]
    highlight_string: String,

    /// Enable TUI module at startup
    #[arg(short = 'm', long)]
    enable_module: bool,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct AppArgs {
    /// Performance mode, don't read dependencies
    #[arg(short = 'n', long)]
    no_tree: bool,

    #[command(flatten)]
    display_args: DisplayArgs,

    /// Path to todo file (and notes sibling directory)
    #[arg(default_value=get_todo_path().unwrap().into_os_string())]
    todo_path: PathBuf,
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
