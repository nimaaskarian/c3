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
mod schedule;
use schedule::Schedule;
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
    removed_names: Vec<String>,
    schedule: Schedule,
}

impl Into<String> for &Todo {
    fn into(self) -> String{
        let done_str = if self.done() {"-"} else {""};
        let note_str = match self.dependency_type {
            DependencyType::None => String::new(),
            _ => format!(">{}", self.dependency_name),
        };
        let schedule_str:String =(&self.schedule).into();

        format!("[{done_str}{}]{note_str} {}{schedule_str}", self.priority, self.message)
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
        match input {
            _ if sscanf!(input, "[{}]>{}.todo {}", priority_string, dependency_name, message).is_ok() => {
                dependency_type = DependencyType::TodoList;
            }
            _ if sscanf!(input, "[{}]>{} {}", priority_string, dependency_name, message).is_ok() => {
                dependency_type = DependencyType::Note;
            }
            _ if sscanf!(input, "[{}] {}", priority_string, message).is_ok() => {
                dependency_type = DependencyType::None;
            }
            _ => return Err(TodoError::ReadFailed),
        }

        let dependencies = TodoList::new();
        let dependency_name = match dependency_type {
            DependencyType::None => String::new(),
            DependencyType::TodoList => Self::todolist_name(&dependency_name),
            DependencyType::Note => dependency_name,
        };

        let priority:i8 = match priority_string.parse() {
            Ok(value) => {
                match value {
                    0.. => value,
                    any => any*-1,
                }
            }
            Err(_) => 0
        };
        
        let schedule = Schedule::match_message(&mut message);
        let mut done = priority_string.chars().nth(0).unwrap() == '-';

        if schedule.should_undone() {
            done = false;
        }
        Ok(Todo {
            note: String::new(),
            dependency_type,
            schedule,
            removed_names: Vec::new(),
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
            schedule: Schedule::new(),
            removed_names: Vec::new(),
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

    pub fn remove_note(&mut self) {
        if self.dependency_type == DependencyType::Note {
            self.remove_dependency();
        }
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
    }

    #[inline]
    pub fn add_todo_dependency(&mut self) -> Result<(), TodoError>{
        if self.has_todo_dependency() {
            return Err(TodoError::AlreadyExists)
        }
        // if let Some(path) = self.dependency_path(path) {
        //     let _ = self.remove_note(&path);
        // }
        self.dependency_name = Self::todolist_name(&self.hash());

        self.dependency_type = DependencyType::TodoList;
        self.dependencies = TodoList::new();

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
        for name in &self.removed_names {
            let _ = remove_file(path.join(name));
        }
        self.removed_names = Vec::new();
        Ok(())
    }

    pub fn has_todo_dependency(&self) -> bool {
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
        let daily_str = self.schedule.display();
        format!("{done_string}{}{note_string} {}{daily_str}", self.priority, self.message)
    }

    pub fn dependency_path(&self ,path: &PathBuf) -> Option<PathBuf>{
        match path.parent() {
            Some(path) => Some(TodoList::dependency_parent(&path.to_path_buf(), false)),
            None => None,
        }
    }

    pub fn remove_dependency(&mut self) {
        if self.dependency_type != DependencyType::None {
            self.removed_names.push(self.dependency_name.clone());
        }

        self.dependency_type = DependencyType::None;
        self.dependencies.remove_dependencies();
        self.dependency_name = String::new();
        self.note = String::new();
    }

    pub fn set_note(&mut self, note:String) -> io::Result<()>{
        self.dependency_name = sha1(&note);
        self.dependency_type = DependencyType::Note;
        self.note = note;
        Ok(())
    }

    pub fn edit_note(&mut self)-> io::Result<()>{
        let note = open_temp_editor(self.note.clone())?;

        self.set_note(note)?;
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
        self.schedule.toggle();
        self.schedule.set_daily();
    }

    #[inline]
    pub fn toggle_weekly(&mut self) {
        self.schedule.toggle();
        self.schedule.set_weekly();
    }

    #[inline]
    pub fn enable_day(&mut self, day: i64) {
        self.schedule.enable();
        self.schedule.set_day(day);
    }

    #[inline]
    pub fn set_done(&mut self, done:bool) {
        if done {
            self.schedule.current_date()
        } else {
            self.schedule.none_date()
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
        todo.set_note("Note".to_string());

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
            schedule: Schedule::new(),
            removed_names: Vec::new(),
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
        todo.add_todo_dependency().expect("Error setting dependency");

        let result = &todo.dependency_name;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_remove_note() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let path = PathBuf::new();
        todo.set_note("Note".to_string()).expect("Error setting note");
        let pathbuf = PathBuf::from("test/TODO_LIST");

        todo.remove_note();

        assert_eq!(todo.dependency_type, DependencyType::None);
        assert!(todo.note_empty());
    }

    #[test]
    fn test_add_dependency() {
        let mut todo = Todo::default("Test".to_string(), 1);

        let pathbuf = PathBuf::from("test/TODO_LIST");

        todo.add_todo_dependency();

        assert!(todo.has_todo_dependency());
    }

    #[test]
    fn test_remove_dependency() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let pathbuf = PathBuf::from("test/TODO_LIST");
        todo.add_todo_dependency();

        todo.remove_dependency();

        assert!(!todo.has_todo_dependency());
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
            schedule: Schedule::new(),
            removed_names: Vec::new(),
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
            schedule: Schedule::from("DAILY 2023-09-05"),
            removed_names: Vec::new(),
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
    }

    #[test]
    fn test_daily_display() {
        let test = Todo {
            note: String::new(),
            dependency_type: DependencyType::None,
            schedule: Schedule::from("DAILY"),
            removed_names: Vec::new(),
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
