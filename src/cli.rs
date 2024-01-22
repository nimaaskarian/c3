use std::io;

mod app;
use app::App;
use crate::Args;

#[inline]
pub fn run(args: Args) -> io::Result<()>{
    let app = App::new(args);
    app.print()?;
    Ok(())
}
