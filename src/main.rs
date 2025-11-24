// vim:fileencoding=utf-8:foldmethod=marker
// imports {{{
use clap::Parser;
use std::io;
pub(crate) mod cli_app;
pub(crate) mod tui_app;
use c3::{todo_app::App, AppArgs};
use cli_app::CliArgs;
use tui_app::TuiArgs;
// }}}

/// A tree-like todo application that makes you smile
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[clap(flatten)]
    pub app_args: AppArgs,

    #[clap(flatten)]
    pub cli_args: CliArgs,

    #[clap(flatten)]
    pub tui_args: TuiArgs,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mut app = App::new(args.app_args);

    if cli_app::run(&mut app, args.cli_args).is_err() {
        let result = tui_app::run(&mut app, args.tui_args);
        tui_app::shutdown()?;
        result
    } else {
        Ok(())
    }
}
