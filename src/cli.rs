use std::io;

mod app;
use app::App;

#[inline]
pub fn run() -> io::Result<()>{
    let app = App::new();
    app.print()?;
    Ok(())
}
