use std::io;

use scanf::scanf;
use cursive;
use cursive::Cursive;
use cursive::theme::{Color, PaletteColor, Theme, BorderStyle};
use cursive::views::{Dialog, LinearLayout, TextView, SelectView};

mod todo_list;
pub mod fileio;

use todo_list::TodoList;
use todo_list::todo::Todo;
use fileio::todo_path;

fn get_message() -> io::Result<String> {
    print!("Message: ");
    let mut message = String::new();
    scanf!("{}", message)?;
    Ok(message)
}

fn get_priority() -> io::Result<i8> {
    print!("Priority: ");
    let mut priority = 0;
    scanf!("{}", priority)?;
    Ok(priority)
}

fn main() -> io::Result<()> {

    let mut select = SelectView::<Todo>::new().on_submit(on_submit);
    let mut siv = cursive::default();

    let mut theme = Theme::default();
    theme.palette[PaletteColor::Background] = Color::TerminalDefault;
    theme.palette[PaletteColor::View] = Color::TerminalDefault;
    theme.palette[PaletteColor::Primary] = Color::TerminalDefault;
    theme.shadow = false;
    theme.borders = BorderStyle::None;
    siv.set_theme(theme);
    let todo_path = todo_path()?;

    let mut todo_list = TodoList::read(&todo_path);
    todo_list.undone.sort();
    for todo in todo_list.undone.todos {
        select.add_item(&todo, todo.clone());
    }
    // Add the dialog to the Cursive root
    siv.add_layer(Dialog::around(LinearLayout::vertical()
    .child(select)));


    // Run the event loop
    siv.run();
    Ok(())
}

fn on_submit(s: &mut Cursive, todo: &Todo) {
    s.pop_layer();
    s.add_layer(Dialog::text(todo)
        .title(format!("Todo string"))
        .button("Quit", Cursive::quit));
}
