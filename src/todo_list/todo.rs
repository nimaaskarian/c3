// vim:fileencoding=utf-8:foldmethod=marker
//std{{{
use std::{io::{self, Write}, path::PathBuf, fs::{File, remove_file}};
use chrono::Duration;
//}}}
// lib{{{
use scanf::sscanf;
// }}}
// mod{{{
mod note;
mod date;
use note::{sha1, open_temp_editor};

use super::TodoList;
//}}}

#[derive(Debug, PartialEq, Clone, Default)]
enum DependencyType {
    #[default]
    None,
    TodoList,
    Note,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Todo {
    note: String,
    pub message: String,
    priority: i8,
    pub dependencies: TodoList,
    dependency_name: String,
    dependency_type: DependencyType,
    done:bool,
    removed_files: Vec<PathBuf>,
    daily: bool,
    date_str: String,
}

impl Into<String> for &Todo {
    fn into(self) -> String{
        let done_str = if self.done() {"-"} else {""};
        let note_str = match self.dependency_type {
            DependencyType::None => String::new(),
            _ => format!(">{}", self.dependency_name),
        };
        let daily_str = if self.daily {
            format!(" [DAILY {}]", self.date_str)
        } else {
            String::new()
        };
        format!("[{done_str}{}]{note_str} {}{daily_str}", self.priority, self.message)
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

    fn try_from(input:&str) -> Result<Todo, TodoError>{
        let mut message = String::new();
        let mut date_string = String::new();
        let mut priority_string:String = String::new();
        let mut dependency_type = DependencyType::None;
        let mut dependency_name = String::new();

        if sscanf!(input,"[{}]>{}.todo {}", priority_string, dependency_name, message).is_err() {
            if sscanf!(input,"[{}]>{} {}", priority_string, dependency_name, message).is_err() {
                if sscanf!(input,"[{}] {}", priority_string, message).is_err() {
                    return Err(TodoError::ReadFailed);
                }
            }
        } else {
            dependency_type = DependencyType::TodoList;
        }

        if dependency_type == DependencyType::None && !dependency_name.is_empty() {
            dependency_type = DependencyType::Note;
        }
        let dependencies = TodoList::new();
        let dependency_name = match dependency_type {
            DependencyType::None => String::new(),
            DependencyType::TodoList => Self::todolist_name(&dependency_name),
            DependencyType::Note => dependency_name,
        };
        let mut done = priority_string.chars().nth(0).unwrap() == '-';

        let priority:i8 = match priority_string.parse() {
            Ok(value) => {
                match value {
                    0.. => value,
                    any => any*-1,
                }
            }
            Err(_) => 0
        };
        
        let daily = if sscanf!(message.clone().as_str(), "{}[DAILY {}]", message, date_string).is_ok() {
            if let Ok(date) = date::parse(&date_string) {
                let current_date = date::current();
                if current_date > date {
                    done = false
                }
            }
            true
        } else {
            false
        };

        let date_str = if daily {
            if done {
                date::current_str()
            } else {
                date_string
            }
        } else {
            String::new()
        };


        Ok(Todo {
            note: String::new(),
            dependency_type,
            date_str,
            daily,
            removed_files: Vec::new(),
            dependency_name,
            dependencies,
            message,
            priority,
            done,
        })
    }
}

impl Todo {
    #[inline]
    pub fn default(message:String, priority:i8) -> Self {
        Self::new(message, priority, false)
    }

    #[inline]
    pub fn new(message:String, priority:i8, done: bool) -> Self {
        Todo {
            note: String::new(),
            dependency_type: DependencyType::None,
            date_str: String::new(),
            daily: false,
            removed_files: Vec::new(),
            dependency_name: String::new(),
            dependencies: TodoList::new(),
            message,
            priority: Todo::fixed_priority(priority),
            done,
        }
    }

    #[inline]
    pub fn note_empty(&self) -> bool {
        self.note.is_empty()
    }

    #[inline]
    pub fn no_dependency(&self) -> bool {
        self.dependency_type == DependencyType::None
    }

    fn todolist_name(name:&String) -> String {
        format!("{name}.todo")
    }

    // pub fn dependency_path(&self) -> Option<PathBuf> {
    //     note_path(&self.dependency_name, self.dependency_dir()).unwrap()
    // }

    pub fn remove_note(&mut self, path: &PathBuf) {
        // if let Some(path) = self.dependency_path() {
        //     self.removed_files.push(path);
        // }
        // if !self.dependency_name.is_empty() {
        if self.dependency_type == DependencyType::Note {
            self.remove_dependency(path);
            // self.removed_files.push(path.join(name));
        }
        self.dependency_type = DependencyType::None;
    }

    #[inline]
    pub fn read_dependencies(&mut self, path: &PathBuf) -> io::Result<()>{
        let name = self.dependency_name.clone();
        match self.dependency_type {
            DependencyType::Note => {
                self.note = std::fs::read_to_string(path.join(&name))?;
            }
            DependencyType::TodoList => {
                self.dependencies = TodoList::read(&path.join(&name), true, false);
            }
            DependencyType::None => {}
        }
        Ok(())
        // self.dependencies.write(&path.join(&self.dependency_name), false)?;
    }

    #[inline]
    pub fn add_dependency(&mut self, path: &PathBuf) -> Result<(), TodoError>{
        if self.has_dependency() {
            return Err(TodoError::AlreadyExists)
        }
        // let _ = self.remove_note(path);
        self.dependency_name = Self::todolist_name(&self.hash());
        // if File::create(&path).is_err() {
        //     return Err(TodoError::DependencyCreationFailed)
        // }

        self.dependency_type = DependencyType::TodoList;
        self.dependencies = TodoList::read(&path, true, false);

        Ok(())
    }

    pub fn dependency_write(&mut self, path: &PathBuf) -> io::Result<()> {
        let name = self.dependency_name.clone();
        match self.dependency_type {
            DependencyType::TodoList =>self.dependencies.write(&path.join(&name), false)?,
            DependencyType::Note => {
                let mut file = File::create(path.join(name))?;
                write!(file, "{}", self.note);
            }
            DependencyType::None => {}
        }
        Ok(())
    }

    pub fn remove_dependent_files(&mut self, path: &PathBuf) -> io::Result<()>{
        self.dependencies.handle_dependent_files(path);
        for path in &self.removed_files {
            let _ = remove_file(path);
        }
        self.removed_files = Vec::new();
        Ok(())
    }

    pub fn has_dependency(&self) -> bool {
        self.dependency_type == DependencyType::TodoList
    }

    pub fn done(&self) -> bool {
        return self.done
    }

    pub fn display(&self, show_done: Option<bool>) -> String {
        let show_done = match show_done {
            None => true,
            Some(value) => value,
        };
        let done_string = if show_done {
            let inside_str = if self.done() {
                "x"
            } else {
                " "
            };
            format!("[{inside_str}] ")
        } else {
            String::new()
        };
        let note_string = match self.dependency_type {
            DependencyType::None => ".",
            DependencyType::Note => ">",
            DependencyType::TodoList => "-",
        };
        let daily_str = if self.daily {
            let inner_str = if self.date_str.is_empty() {
                String::new()
            } else {
                let last_save = if let Ok(parsed_date) = date::parse(&self.date_str) {
                    date::current() - parsed_date
                } else {
                    Duration::zero()
                };
                match last_save.num_days() {
                    0 => String::new(),
                    1 => String::from(", last done yesterday"),
                    any => format!(", last done {} days ago", any)
                }
            };
            format!(" (Daily{inner_str})")
        } else {
            String::new()
        };
        format!("{done_string}{}{note_string} {}{daily_str}", self.priority, self.message)
    }

    pub fn dependency_path(&self ,path : &PathBuf) -> Option<PathBuf>{
        match path.parent() {
            Some(path) => Some(path.to_path_buf().join("notes").join(&self.dependency_name).clone()),
            None => None,
        }
    }

    pub fn remove_dependency(&mut self, path: &PathBuf) {
        if let Some(path) = self.dependency_path(path) {
            self.removed_files.push(path);
        }

        self.dependency_type = DependencyType::None;
        self.dependencies.remove_dependencies(path);
        self.dependency_name = String::new();
        self.note = String::new();
        // self.dependencies = TodoList::new();
    }

    pub fn set_note(&mut self, note:String, path: &PathBuf) -> io::Result<()>{
        // let _ = self.remove_dependency(&path);
        self.dependency_name = sha1(&note);
        self.dependency_type = DependencyType::Note;
        self.note = note;
        Ok(())
    }

    pub fn edit_note(&mut self, path: &PathBuf)-> io::Result<()>{
        let note = open_temp_editor(self.note.clone())?;

        self.set_note(note, path)?;
        Ok(())
    }

    #[inline]
    pub fn get_note_content(&self) -> &String {
        return &self.note
    }

    #[inline]
    pub fn set_message(&mut self, message:String) {
        self.message = message;
    }

    #[inline]
    pub fn hash(&self) -> String{
        sha1(&format!("{} {}", self.priority, self.message))
    }

    #[inline]
    pub fn toggle_done(&mut self) {
        self.set_done(!self.done);
    }

    #[inline]
    pub fn toggle_daily(&mut self) {
        self.daily = !self.daily;
    }

    #[inline]
    pub fn set_done(&mut self, done:bool) {
        if self.daily && done {
            self.date_str = date::current_str();
        } else {
            self.date_str = String::new();
        }
        self.done = done;
    }


    #[inline]
    pub fn decrease_priority(&mut self) {
        if self.comparison_priority() < 9 {
            self.priority+=1
        } else {
            self.priority=0
        }
    }

    #[inline]
    pub fn increase_priority(&mut self) {
        if self.comparison_priority() > 1 {
            self.priority=self.comparison_priority()-1
        } else {
            self.priority=1
        }
    }

    #[inline]
    pub fn set_priority(&mut self, priority:i8) {
        self.priority = priority;
        self.fix_priority();
    }

    #[inline]
    fn fix_priority(&mut self) {
        self.priority = Todo::fixed_priority(self.priority)
    }

    #[inline(always)]
    pub fn comparison_priority(&self) -> i8{
        if self.priority == 0 {10} else {self.priority}
    }

    #[inline]
    fn fixed_priority(priority: i8) -> i8 {
        match priority {
            10.. => 0,
            0 => 0,
            ..=0 => 1,
            _ => priority
        }
    }

    #[inline]
    pub fn as_string(&self) -> String{
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::fileio::append_home_dir;
    use crate::fileio::note_path;

    use super::*;

    #[test]
    fn test_todo_into_string() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let path = PathBuf::new();
        todo.set_note("Note".to_string(), &path);

        let expected = "[1]>2c924e3088204ee77ba681f72be3444357932fca Test";
        let result: String = (&todo).into();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_try_from_string() {
        let input = "[1]>2c924e3088204ee77ba681f72be3444357932fca Test";
        let expected = Ok(Todo {
            note: String::new(),
            dependency_type: DependencyType::Note,
            date_str: String::new(),
            daily: false,
            removed_files: Vec::new(),
            dependency_name: String::from("2c924e3088204ee77ba681f72be3444357932fca"),
            message: "Test".to_string(),
            priority: 1,
            dependencies: TodoList::new(),
            done: false,
        });

        let result: Result<Todo, TodoError> = Todo::try_from(input.to_string());

        assert_eq!(result, expected);
    }

    #[test]
    fn test_new_todo() {
        let message = "New Todo";
        let priority = 2;

        let todo = Todo::default(message.to_string(), priority);

        assert_eq!(todo.message, message);
        assert_eq!(todo.priority, 2);
        assert_eq!(todo.dependencies, TodoList::new());
        assert_eq!(todo.dependency_name, String::new());
        assert_eq!(todo.done, false);
    }

    #[test]
    fn test_static_dependency_name() {
        let name = "my_dep".to_string();
        let expected = "my_dep.todo".to_string();

        let result = Todo::todolist_name(&name);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_default_dependency_path() {
        let name = "my_dep".to_string();
        let expected = PathBuf::from(append_home_dir(".local/share/calcurse/notes/my_dep.todo"));

        let result = note_path(&Todo::todolist_name(&name), None).unwrap().unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_dependency_name() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let expected = "900a80c94f076b4ee7006a9747667ccf6878a72b.todo";
        let pathbuf = PathBuf::from("tests/TODO_LIST");
        todo.add_dependency(&pathbuf).expect("Error setting dependency");

        let result = &todo.dependency_name;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_remove_note() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let path = PathBuf::new();
        todo.set_note("Note".to_string(), &path).expect("Error setting note");
        let pathbuf = PathBuf::from("test/TODO_LIST");

        todo.remove_note(&pathbuf);

        assert_eq!(todo.dependency_type, DependencyType::None);
        assert!(todo.note_empty());
    }

    #[test]
    fn test_add_dependency() {
        let mut todo = Todo::default("Test".to_string(), 1);

        let pathbuf = PathBuf::from("test/TODO_LIST");

        todo.add_dependency(&pathbuf);

        assert!(todo.has_dependency());
    }

    #[test]
    fn test_remove_dependency() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let pathbuf = PathBuf::from("test/TODO_LIST");
        todo.add_dependency(&pathbuf);

        todo.remove_dependency(&pathbuf);

        assert!(!todo.has_dependency());
    }

    #[test]
    fn test_toggle_done() {
        let mut todo = Todo::default("Test".to_string(), 1);

        todo.toggle_done();
        assert_eq!(todo.done(), true);

        todo.toggle_done();
        assert_eq!(todo.done(), false);
    }

    #[test]
    fn test_from_string() {
        let input1 = "[1]>1BE348656D84993A6DF0DB0DECF2E95EF2CF461c.todo Read for exams";
        let todo = Todo::try_from(input1).unwrap();

        let expected = Todo {
            note: String::new(),
            dependency_type: DependencyType::TodoList,
            date_str: String::new(),
            daily: false,
            removed_files: Vec::new(),
            dependency_name: "1BE348656D84993A6DF0DB0DECF2E95EF2CF461c.todo".to_string(),
            message: "Read for exams".to_string(),
            priority: 1,
            dependencies: TodoList::new(),
            done: false,
        };
        assert_eq!(todo, expected);
    }

    #[test]
    fn test_daily() {
        let input = "[-2] this one should be daily [DAILY 2023-09-05]";
        let todo = Todo::try_from(input).unwrap();
        let expected = Todo {
            note: String::new(),
            dependency_type: DependencyType::None,
            date_str: "2023-09-05".to_string(),
            daily: true,
            removed_files: Vec::new(),
            dependency_name: String::new(),
            message: "this one should be daily".to_string(),
            priority: 2,
            dependencies: TodoList::new(),
            done: false,
        };
        assert_eq!(todo, expected);
        let input = "[2] this one should be daily [DAILY 2023-09-05]";
        let todo = Todo::try_from(input).unwrap();
        assert_eq!(todo, expected);

        let test = Todo {
            note: String::new(),
            dependency_type: DependencyType::None,
            date_str: String::new(),
            daily: true,
            removed_files: Vec::new(),
            dependency_name: String::new(),
            message: "this one should be daily".to_string(),
            priority: 2,
            dependencies: TodoList::new(),
            done: false,
        };
        let expected = "2. this one should be daily (Daily)";

        assert_eq!(test.display(Some(false)), expected)
    }
}
