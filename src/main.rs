mod todo_list;
use todo_list::TodoList;
use std::path::PathBuf;
use std::time::Instant;
use std::env;

const TODO_DIR:&str = ".local/share/calcurse/";
const TODO_FILE:&str = "todo";

fn main() -> std::io::Result<()>{
    let home = env::var("HOME").unwrap();
    let mut todo_file = PathBuf::from(home).join(TODO_DIR).join(TODO_FILE);
    if let Some(parent) = todo_file.parent() {
        std::fs::create_dir_all(parent.join("note"))?;
    }

    let mut todo_list = TodoList::read(&todo_file);
    todo_list.write("./todo").expect("file write error");
    todo_list.undone.sort();
    todo_list.undone.print();

    todo_list.undone.reorder(2);

    Ok(())
}
