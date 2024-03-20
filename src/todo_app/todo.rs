// vim:fileencoding=utf-8:foldmethod=marker
//std{{{
use std::{io, path::PathBuf, fs::remove_file};
//}}}
// lib{{{
use scanf::sscanf;
// }}}
// mod{{{
mod note;
mod schedule;
mod dependency;
use dependency::Dependency;
use schedule::Schedule;
use note::{sha1, open_temp_editor};

use super::TodoList;
//}}}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Todo {
    pub message: String,
    priority: i8,
    pub dependency: Dependency,
    removed_dependencies: Vec<Dependency>,
    done:bool,
    pub schedule: Schedule,
}

impl Into<String> for &Todo {
    fn into(self) -> String{
        let done_str = if self.done() {"-"} else {""};
        let dep_str:String = (&self.dependency).into();

        let schedule_str:String =(&self.schedule).into();

        format!("[{done_str}{}]{dep_str} {}{schedule_str}", self.priority, self.message)
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

    fn try_from(s:String) -> Result<Todo, Self::Error>{
        Todo::try_from(s.as_str())
    }
}

impl TryFrom<&str> for Todo {
    type Error = TodoError;

    fn try_from(input:&str) -> Result<Todo, Self::Error>{
        let mut message = String::new();
        let mut priority_string:String = String::new();
        let mut dependency_string = String::new();

        match input {
            _ if sscanf!(input, "[{}]>{} {}", priority_string, dependency_string, message).is_ok() => {},

            _ if sscanf!(input, "[{}] {}", priority_string, message).is_ok() => {
                dependency_string = String::new();
            }
            _ => return Err(Self::Error::ReadFailed),
        }
        let dependency = Dependency::from(dependency_string.as_str());

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
            dependency,
            removed_dependencies: vec![],
            schedule,
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
            schedule: Schedule::new(),
            dependency: Dependency::default(),
            removed_dependencies: vec![],
            message,
            priority: Todo::fixed_priority(priority),
            done,
        }
    }

    #[inline]
    pub fn note_empty(&self) -> bool {
        self.dependency.is_note()
    }

    #[inline]
    pub fn dependencies(&self) -> Option<&TodoList> {
        self.dependency.todo_list()
    }

    #[inline]
    pub fn no_dependency(&self) -> bool {
        self.dependency.is_none()
    }

    #[inline]
    pub fn remove_note(&mut self) {
        if self.dependency.is_note() {
            self.remove_dependency();
        }
    }

    #[inline]
    pub fn add_todo_dependency(&mut self) -> Result<(), TodoError>{
        if self.dependency.is_none() {
            self.dependency = Dependency::new_todo_list(self.hash());
            Ok(())
        } else {
            Err(TodoError::AlreadyExists)
        }
    }

    #[inline]
    pub fn delete_dependency_file(&mut self, path: &PathBuf) -> io::Result<()> {
        self.dependency.todo_list.remove_dependency_files(path)?;
        let _ = remove_file(path.join(self.dependency.get_name()));
        Ok(())
    }

    #[inline]
    pub fn delete_removed_dependent_files(&mut self, path: &PathBuf) -> io::Result<()>{
        for dependency in &mut self.removed_dependencies {
            let _ = dependency.todo_list.remove_dependency_files(path);
            let _ = remove_file(path.join(dependency.get_name()));
        }
        Ok(())
    }

    #[inline]
    pub fn done(&self) -> bool {
        self.done
    }

    #[inline]
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
        let note_string = self.dependency.display();
        let daily_str = self.schedule.display();
        format!("{done_string}{}{note_string} {}{daily_str}", self.priority, self.message)
    }

    #[inline]
    pub fn remove_dependency(&mut self) {
        self.removed_dependencies.push(self.dependency.clone());
        self.dependency.remove();
    }

    #[inline]
    pub fn set_note(&mut self, note:String) -> io::Result<()>{
        self.dependency = Dependency::new_note(sha1(&note), note);
        Ok(())
    }

    #[inline]
    pub fn edit_note(&mut self)-> io::Result<()>{
        if !self.dependency.is_list() {
            let note = open_temp_editor(self.dependency.note())?;
            if !note.is_empty() {
                self.set_note(note)?;
            }
        }
        Ok(())
    }

    #[inline]
    pub fn dependency_path(&self, path: &PathBuf) -> Option<PathBuf> {
        self.dependency.path(path)
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
    use super::*;

    #[test]
    fn test_todo_into_string() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let _ = todo.set_note("Note".to_string());

        let expected = "[1]>2c924e3088204ee77ba681f72be3444357932fca Test";
        let result: String = (&todo).into();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_try_from_string() {
        let input = "[1]>2c924e3088204ee77ba681f72be3444357932fca Test";
        let expected = Ok(Todo {
            removed_dependencies: vec![],
            schedule: Schedule::new(),
            dependency:Dependency::new_note("2c924e3088204ee77ba681f72be3444357932fca".to_string(), "".to_string()),
            message: "Test".to_string(),
            priority: 1,
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
        assert_eq!(todo.done, false);
    }

    #[test]
    fn test_dependency_name() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let expected = "900a80c94f076b4ee7006a9747667ccf6878a72b.todo";
        todo.add_todo_dependency().expect("Error setting dependency");

        let result = &todo.dependency.get_name();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_dependency_type() {
        let mut todo = Todo::default("Test".to_string(), 1);
        todo.add_todo_dependency().expect("Error setting dependency");

        assert!(todo.dependency.is_list());
    }

    #[test]
    fn test_add_todo() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let expected = "900a80c94f076b4ee7006a9747667ccf6878a72b.todo";
        todo.add_todo_dependency().expect("Error setting dependency");

        let result = &todo.dependency.get_name();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_add_note_type() {
        let mut todo = Todo::default("Test".to_string(), 1);
        todo.set_note("Note".to_string()).expect("Error setting note");

        assert!(todo.dependency.is_note());
    }

    #[test]
    fn test_add_note_name() {
        let mut todo = Todo::default("Test".to_string(), 1);
        todo.set_note("Note".to_string()).expect("Error setting note");

        assert_eq!(todo.dependency.get_name(), "2c924e3088204ee77ba681f72be3444357932fca");
    }

    #[test]
    fn test_remove_note() {
        let mut todo = Todo::default("Test".to_string(), 1);
        todo.set_note("Note".to_string()).expect("Error setting note");
        todo.remove_note();

        assert!(todo.dependency.is_none());
    }

    #[test]
    fn test_add_dependency() {
        let mut todo = Todo::default("Test".to_string(), 1);

        let _ = todo.add_todo_dependency();

        assert!(todo.dependency.is_list());
    }

    #[test]
    fn test_remove_dependency() {
        let mut todo = Todo::default("Test".to_string(), 1);
        let _ = todo.add_todo_dependency();

        todo.remove_dependency();

        assert!(!todo.dependency.is_list());
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
            removed_dependencies: vec![],
            schedule: Schedule::new(),
            dependency: Dependency::new_todo_list( "1BE348656D84993A6DF0DB0DECF2E95EF2CF461c".to_string()),
            message: "Read for exams".to_string(),
            priority: 1,
            done: false,
        };
        assert_eq!(todo, expected);
    }

    #[test]
    fn test_daily() {
        let input = "[-2] this one should be daily [D1(2023-09-05)]";
        let todo = Todo::try_from(input).unwrap();
        let expected = Todo {
            removed_dependencies: vec![],
            schedule: Schedule::from("D1(2023-09-05)"),
            dependency: Dependency::default(),
            message: "this one should be daily".to_string(),
            priority: 2,
            done: false,
        };
        assert_eq!(todo, expected);
        let input = "[2] this one should be daily [D1(2023-09-05)]";
        let todo = Todo::try_from(input).unwrap();
        assert_eq!(todo, expected);
    }

    #[test]
    fn test_daily_display() {
        let test = Todo {
            removed_dependencies: vec![],
            dependency: Dependency::default(),
            schedule: Schedule::from("D1()"),
            message: "this one should be daily".to_string(),
            priority: 2,
            done: false,
        };
        let expected = "2. this one should be daily (Daily)";

        assert_eq!(test.display(Some(false)), expected)
    }

    #[test]
    fn test_weekly() {
        let input = "[-2] this one should be daily [D7(2023-09-05)]";
        let todo = Todo::try_from(input).unwrap();
        let expected = Todo {
            removed_dependencies: vec![],
            dependency: Dependency::default(),
            schedule: Schedule::from("D7(2023-09-05)"),
            message: "this one should be daily".to_string(),
            priority: 2,
            done: false,
        };
        assert_eq!(todo, expected);
        let input = "[2] this one should be daily [D7(2023-09-05)]";
        let todo = Todo::try_from(input).unwrap();
        assert_eq!(todo, expected);
    }
}
