mod todo_list;
use todo_list::TodoList;
use scanf::scanf;
pub mod fileio;

use fileio::append_home_dir;

const TODO_PATH:&str = ".local/share/calcurse/todo";

fn get_message() -> String {
    print!("Message: ");
    let mut message = String::new();
    scanf!("{}", message);
    return message
}

fn get_priority() -> i8 {
    print!("Priority: ");
    let mut priority = 0;
    scanf!("{}", priority);
    return priority
}

fn main() -> std::io::Result<()>{
    let todo_path = append_home_dir(TODO_PATH);
    println!("{:?}", todo_path.as_os_str());
    if let Some(parent) = todo_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // todo_list.undone.sort();
    // todo_list.undone.print();

    // todo_list.undone.reorder(2);

    // Ok(())
    let mut todo_list = TodoList::read(&todo_path);
    todo_list.undone.sort();
    let mut resume_char = '\0';
    while resume_char != 'n' {
        if todo_list.undone.len() != 0 {
            todo_list.undone.sort();
            println!("Your todo list:");
            todo_list.undone.print();
        } else {
            println!("You have no todo.")
        }
        let mut add_todo_char = 'n';
        print!("You want to add a todo? [y/N] ");
        scanf!("{}", add_todo_char);
        if add_todo_char == 'y' {
            todo_list.add(get_message(), get_priority());
            continue;
        }
        let mut number = 0;

        print!("Number: ");
        if scanf!("{}", number).is_err() {
            continue;
        }
        if number == 0 {
            continue;
        }
        let todo = &mut todo_list.undone[number-1];

        loop {
            println!("{}", todo.as_string());
            println!("1: Print todo
2: Set message
3: Set priority
4: Add note
5: View note
6: Edit note
7: Quit todo");
            let mut command = 0;
            print!("Command: ");
            scanf!("{}", command);
            match command {
                1 => println!("{}", todo.as_string()),
                2 => {
                    todo.set_message(get_message())
                }
                3 => {
                    todo.set_priority(get_priority())
                }
                4 => {
                    todo.add_note();
                }
                5 => {
                    println!("Note: '{}'", todo.note())
                }
                6 => {
                    todo.edit_note();
                }
                7 => {
                    break;
                }
                _ =>{}
            }
        }

        print!("Do you want to select another todo? [Y/n] ");
        scanf!("{}", resume_char);
    }

    let mut save_char = 'y';
    print!("Do you want to save? [Y/n] ");
    scanf!("{}", save_char);

    if save_char.to_ascii_lowercase() == 'y' {
        todo_list.write(todo_path.to_str().unwrap()).expect("file write error");
        print!("Saved. ");
    }
    println!("Goodbye!");
    Ok(())
}
