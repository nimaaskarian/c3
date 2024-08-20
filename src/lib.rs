use clap::{Parser, ValueEnum};
use todo_app::SortMethod;
use std::path::PathBuf;
use fileio::get_todo_path;
use std::fmt;

pub mod date;
pub mod fileio;
pub mod todo_app;

#[derive(ValueEnum, Clone, Debug)]
pub enum DoOnSelected {
    Delete,
    Done,
}


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct AppArgs {
    /// Performance mode, don't read dependencies
    #[arg(short = 'n', long)]
    pub no_tree: bool,

    #[command(flatten)]
    pub display_args: DisplayArgs,

    /// Path to todo file (and notes sibling directory)
    #[arg(default_value=get_todo_path().unwrap().into_os_string())]
    pub todo_path: PathBuf,

    /// Sort method, how sortings are done in the app
    #[arg(long, default_value = "normal")]
    pub sort_method: SortMethod,
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

pub trait TodoDisplay: fmt::Display {
    fn display_with_args(&self, args: &DisplayArgs) -> String;
}

