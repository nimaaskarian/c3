use scanf::sscanf;
mod note;
use std::{io, path::PathBuf, ops::Add, fs::File};
use note::{Note, sha1};

use crate::fileio::{note_path};

use super::TodoList;

#[derive(Debug, Clone, Default)]
pub struct Todo {
    pub message: String,
    note: String,
    priority: i8,
    pub dependencies: TodoList,
    dependency_path: PathBuf,
    done:bool,
}

impl Into<String> for &Todo {
    fn into(self) -> String{
        let done_str = if self.done() {"-"} else {""};
        self.hash();
        let mut note_str = match self.note.as_str() {
            "" => String::new(),
            _ => format!(">{}", self.note),
        };
        if self.has_dependency() {
            note_str = format!(">{}", self.dependency_name());
        }
        format!("[{done_str}{}]{note_str} {}", self.priority, self.message)
    }
}

#[derive(Debug, PartialEq)]
pub enum TodoError {
    ReadFailed,
    NoteEmpty,
    AlreadyExists,
    DependencyCreationFailed,
}

impl TryFrom<String> for Todo {
    type Error = TodoError;

    fn try_from(s:String) -> Result<Todo, TodoError>{
        Todo::try_from(s.as_str())
    }
}

impl TryFrom<&str> for Todo {
    type Error = TodoError;

    fn try_from(s:&str) -> Result<Todo, TodoError>{
        let mut message = String::new();
        let mut note = String::new();
        let mut todo = String::new();
        let mut priority_string:String = String::new();

        if sscanf!(s,"[{}]>{}.todo {}", priority_string, todo, message).is_err() {
            if sscanf!(s,"[{}]>{} {}", priority_string, note, message).is_err() {
                if sscanf!(s,"[{}] {}", priority_string, message).is_err() {
                    return Err(TodoError::ReadFailed);
                }
            }
        }
        let mut dependency_path = PathBuf::new();
        let mut dependencies = TodoList::new();
        if todo != "" {
            dependency_path = Self::static_dependency_path(todo);
            dependencies = TodoList::read(&dependency_path);
        }

        let done = priority_string.chars().nth(0).unwrap() == '-';
        let mut priority:i8 = priority_string.parse().unwrap();

        if done {
            priority*=-1;
        }
        Ok(Todo {
            dependency_path,
            dependencies,
            message,
            note,
            priority,
            done,
        })
    }
}

impl Todo {
    pub fn new(message:String, priority:i8) -> Self {
        Todo {
            dependency_path: PathBuf::new(),
            note: String::new(),
            dependencies: TodoList::new(),
            message,
            priority: Todo::fixed_priority(priority),
            done: false,
        }
    }

    fn static_dependency_name(name:String) -> String {
        format!("{name}.todo")
    }

    fn static_dependency_path(name:String) -> PathBuf {
        return note_path(&Self::static_dependency_name(name)).unwrap();
    }

    fn dependency_name(&self) -> String {
        format!("{}.todo", self.hash())
        // Self::static_dependency_name(self.hash())
    }

    pub fn dependency_path(&self) -> PathBuf {
        return Self::static_dependency_path(self.hash())
    }

    pub fn add_dependency(&mut self) -> Result<(), TodoError>{
        if self.has_dependency() {
            return Err(TodoError::AlreadyExists)
        }

        self.dependency_path = self.dependency_path();
        if File::create(&self.dependency_path).is_err() {
            return Err(TodoError::DependencyCreationFailed)
        }

        self.note = String::new();

        self.dependencies = TodoList::read(&self.dependency_path);
        // self.dependencies.add(String::from("Sub to-do"), 0);
        Ok(())

    }

    pub fn has_dependency(&self) -> bool {
        match self.dependency_path.to_str() {
            Some(value) => value != "",
            None => false,
        }
        // self.dependencies.undone.len() != 0
    }

    pub fn done(&self) -> bool {
        if self.has_dependency() {
            return self.dependencies.undone.len() == 0;
        }
        return self.done
    }

    pub fn display(&self) -> String {
        let done_string = if self.done() {
            "x"
        } else {
            " "
        };
        let note_string = if self.note != "" {
            ">"
        } else if self.has_dependency() {
            "-"
        }
        else {
            " "
        };
        format!("[{done_string}] [{}]{note_string}{}", self.priority, self.message)
    }

    pub fn add_note(&mut self)-> io::Result<()>{
        self.dependency_path = PathBuf::new();
        self.dependencies = TodoList::new();
        let note = Note::from_editor()?;

        self.note = note.hash();
        note.save().expect("Note saving failed");
        Ok(())
    }

    pub fn edit_note(&mut self)-> io::Result<()>{
        let mut note = Note::from_hash(&self.note)?;
        note.edit_with_editor()?;
        self.note = note.hash();
        note.save().expect("Note saving failed");
        Ok(())
    }

    pub fn note(&self) -> String {
        match Note::from_hash(&self.note) {
            Err(_) => return String::new(),
            Ok(note) => note.content()
        }
    }

    pub fn set_message(&mut self, message:String) {
        self.message = message;
    }

    pub fn hash(&self) -> String{
        sha1(&format!("{} {}", self.priority, self.message))
    }

    pub fn toggle_done(&mut self) {
        self.done = !self.done;
    }

    pub fn decrease_priority(&mut self) {
        if self.comparison_priority() < 9 {
            self.priority+=1
        } else {
            self.priority=0
        }
    }

    pub fn increase_priority(&mut self) {
        if self.comparison_priority() > 1 {
            self.priority=self.comparison_priority()-1
        } else {
            self.priority=1
        }
    }

    pub fn set_priority(&mut self, add:i8) {
        self.priority = add;
        self.fix_priority();
    }

    fn fix_priority(&mut self) {
        self.priority = Todo::fixed_priority(self.priority)
    }

    #[inline(always)]
    pub fn comparison_priority(&self) -> i8{
        if self.priority == 0 {10} else {self.priority}
    }

    fn fixed_priority(priority: i8) -> i8 {
        match priority {
            10.. => 0,
            0 => 0,
            ..=0 => 1,
            _ => priority
        }
    }

    pub fn as_string(&self) -> String{
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_todo() {
        let todo = Todo::new("Buy groceries".to_string(), 5);
        assert_eq!(todo.message, "Buy groceries");
        assert_eq!(todo.priority, 5);
        assert_eq!(todo.done, false);
    }

    #[test]
    fn test_toggle_done() {
        let mut todo = Todo::new("Buy groceries".to_string(), 5);
        assert_eq!(todo.done, false);

        todo.toggle_done();
        assert_eq!(todo.done, true);

        todo.toggle_done();
        assert_eq!(todo.done, false);
    }

    #[test]
    fn test_set_priority() {
        let mut todo = Todo::new("Buy groceries".to_string(), 5);
        assert_eq!(todo.priority, 5);

        todo.set_priority(8);
        assert_eq!(todo.priority, 8);

        todo.set_priority(2);
        assert_eq!(todo.priority, 2);
    }

    #[test]
    fn test_fix_priority() {
       let mut todo = Todo::new("Buy groceries".to_string(), -1); 
       todo.fix_priority();
       assert_eq!(todo.priority, 0);

       let mut todo = Todo::new("Buy groceries".to_string(), 10); 
       todo.fix_priority();
       assert_eq!(todo.priority, 9);

       let mut todo = Todo::new("Buy groceries".to_string(), 4); 
       todo.fix_priority();
       assert_eq!(todo.priority, 4);  
   }
    #[test]
    fn test_into_string() {
        let mut todo = Todo {
            done: true,
            note: String::from("Note very gud"),
            priority: 1,
            message: String::from("Important job :D"),
        };

        let result: String = todo.as_string();

        assert_eq!(result, "[-1]>Note very gud Important job :D");

        todo.toggle_done();
        let result: String = todo.as_string();
        assert_eq!(result, "[1]>Note very gud Important job :D");
    }
     #[test]
    fn test_try_from_string() {
        let input = String::from("[2]>Note Message");

        let result = Todo::try_from(input).unwrap();

        assert_eq!(
            result,
            Todo {
                done: false,
                note: String::from("Note"),
                priority: 2,
                message: String::from("Message"),
            }
        );
    }

    #[test]
    fn test_try_from_string_with_invalid_input() {
        let input = String::from("[invalid] Invalid Message");

        let result = Todo::try_from(input);

        assert_eq!(result, Err(TodoError::ReadFailed));
    }
}
