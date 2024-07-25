use std::path::Path;
// vim:fileencoding=utf-8:foldmethod=marker
//std{{{
use std::str::{self, FromStr};
use std::{fs::remove_file, io, path::PathBuf};
//}}}
// mod{{{
mod dependency;
mod note;
pub mod schedule;
use super::TodoList;
use crate::DisplayArgs;
use dependency::Dependency;
use note::{open_note_temp_editor, sha1};
use schedule::Schedule;
//}}}

pub type PriorityType = u8;
#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct Todo {
    pub message: String,
    priority: PriorityType,
    pub dependency: Dependency,
    removed_dependency: Option<Dependency>,
    done: bool,
    pub schedule: Schedule,
}

impl Ord for Todo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp_value().cmp(&other.cmp_value())
    }
}

impl PartialOrd for Todo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&Todo> for String {
    fn from(todo: &Todo) -> String {
        let done_str = if todo.done() { "-" } else { "" };
        let dep_str: String = (&todo.dependency).into();

        let schedule_str: String = (&todo.schedule).into();

        format!(
            "[{done_str}{}]{dep_str} {}{schedule_str}",
            todo.priority, todo.message
        )
    }
}

#[derive(Debug, PartialEq)]
pub enum TodoError {
    ReadFailed,
    NoteEmpty,
    DependencyCreationFailed,
}

#[derive(Default, PartialEq)]
enum State {
    #[default]
    Priority,
    Dependency,
    Message,
}

impl FromStr for Todo {
    type Err = TodoError;

    fn from_str(input: &str) -> Result<Todo, Self::Err> {
        let mut state = State::default();
        let mut priority: u8 = 0;
        let mut done = false;
        let mut dependency_string = String::new();
        let mut schedule_string = String::new();
        let mut message = String::new();
        let mut schedule_start_index: Option<usize> = None;
        if input.ends_with(']') {
            schedule_start_index = input.rfind('[');
            if let Some(start) = schedule_start_index {
                let end = input.chars().count();
                schedule_string = input[start + 1..end - 1].chars().collect();
            }
        }

        for (i, c) in input.chars().enumerate() {
            match state {
                State::Priority => {
                    if c == '-' {
                        done = true;
                    } else if c.is_ascii_digit() {
                        priority = c.to_digit(10).unwrap() as PriorityType;
                    } else if c == ' ' {
                        state = State::Message;
                    } else if c == '>' {
                        state = State::Dependency;
                    }
                }
                State::Dependency => {
                    if c == ' ' {
                        state = State::Message;
                    } else {
                        dependency_string.push(c);
                    }
                }
                State::Message if schedule_start_index.is_none() => {
                    message.push(c);
                }
                State::Message => {
                    if i == schedule_start_index.unwrap() - 1 {
                        break;
                    } else {
                        message.push(c);
                    }
                }
            }
        }
        if state == State::Message && !message.is_empty() {
            let schedule = Schedule::from(schedule_string);
            let dependency = Dependency::from(dependency_string.as_str());

            if schedule.should_undone() {
                done = false;
            }
            if schedule.should_done() {
                done = true;
            }
            Ok(Todo {
                dependency,
                removed_dependency: None,
                schedule,
                message,
                priority,
                done,
            })
        } else {
            Err(TodoError::ReadFailed)
        }
    }
}

impl Todo {
    #[inline]
    pub fn matches(&self, query: &str) -> bool {
        self.message.contains(query) || self.message.to_lowercase().contains(query)
    }

    #[inline]
    pub fn priority(&self) -> PriorityType {
        self.priority
    }
    #[inline]
    pub fn new(message: String, priority: PriorityType) -> Self {
        Todo {
            message,
            priority: Self::fixed_priority(priority),
            ..Default::default()
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
    pub fn add_todo_dependency(&mut self) {
        if self.dependency.is_none() {
            self.dependency = Dependency::new_todo_list(self.hash());
        }
    }

    #[inline]
    pub fn delete_dependency_file(&mut self, path: &Path) -> io::Result<()> {
        self.dependency.todo_list.remove_dependency_files(path)?;
        let _ = remove_file(path.join(self.dependency.get_name()));
        Ok(())
    }

    #[inline]
    pub fn delete_removed_dependent_files(&mut self, path: &Path) -> io::Result<()> {
        if let Some(dependency) = &mut self.removed_dependency {
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
    pub fn display(&self, args: &DisplayArgs) -> String {
        let done_string = if args.show_done {
            if self.done() {
                args.done_string.as_str()
            } else {
                args.undone_string.as_str()
            }
        } else {
            ""
        };
        let note_string = self.dependency.display();
        let daily_str = self.schedule.display();
        format!(
            "{done_string}{}{note_string} {}{daily_str}",
            self.priority, self.message
        )
    }

    #[inline]
    pub fn remove_dependency(&mut self) {
        if self.dependency.is_written() {
            self.removed_dependency = Some(self.dependency.clone());
        }
        self.dependency = Dependency::default();
    }

    #[inline]
    pub fn set_note(&mut self, note: String) -> io::Result<()> {
        self.dependency = Dependency::new_note(sha1(&note), note);
        Ok(())
    }

    #[inline]
    pub fn edit_note(&mut self) -> io::Result<()> {
        if !self.dependency.is_list() {
            let note = open_note_temp_editor(self.dependency.note())?;
            if !note.is_empty() {
                self.set_note(note)?;
            }
        }
        Ok(())
    }

    #[inline]
    pub fn dependency_path(&self, path: &Path) -> Option<PathBuf> {
        self.dependency.path(path)
    }

    #[inline]
    pub fn set_message(&mut self, message: String) {
        self.message = message;
    }

    #[inline]
    pub fn hash(&self) -> String {
        sha1(&format!("{} {}", self.priority, self.message))
    }

    #[inline]
    pub fn toggle_done(&mut self) {
        self.set_done(!self.done);
    }

    #[inline]
    pub fn toggle_daily(&mut self) {
        if self.schedule.is_daily() {
            self.schedule.toggle();
        } else {
            self.schedule.enable_schedule();
        }
        self.schedule.set_daily();
    }

    #[inline]
    pub fn toggle_weekly(&mut self) {
        if self.schedule.is_weekly() {
            self.schedule.toggle();
        } else {
            self.schedule.enable_schedule();
        }
        self.schedule.set_weekly();
    }

    #[inline]
    pub fn enable_day(&mut self, day: i64) {
        self.schedule.enable_schedule();
        self.schedule.set_day(day);
    }

    #[inline]
    pub fn set_done(&mut self, done: bool) {
        self.schedule.set_current_date();
        self.done = done;
    }

    #[inline(always)]
    fn standardize_priority(priority: PriorityType) -> PriorityType {
        match priority {
            0 => 10,
            any => any,
        }
    }

    #[inline(always)]
    fn standardized_priority(&self) -> PriorityType {
        Self::standardize_priority(self.priority)
    }

    #[inline]
    pub fn decrease_priority(&mut self) {
        if self.standardized_priority() < 9 {
            self.priority += 1
        } else {
            self.priority = 0
        }
    }

    #[inline]
    pub fn increase_priority(&mut self) {
        if self.standardized_priority() > 1 {
            self.priority = self.standardized_priority() - 1
        } else {
            self.priority = 1
        }
    }

    #[inline]
    pub fn set_priority(&mut self, priority: PriorityType) {
        self.priority = priority;
        self.fix_priority();
    }

    #[inline]
    fn fix_priority(&mut self) {
        self.priority = Todo::fixed_priority(self.priority)
    }

    #[inline(always)]
    fn cmp_value(&self) -> PriorityType {
        let mut priority = self.standardized_priority() * 2;
        if self.schedule.is_reminder() {
            priority -= 1;
        }
        if self.done() {
            priority += Self::standardize_priority(0) * 2;
        }

        priority
    }

    #[inline]
    fn fixed_priority(priority: PriorityType) -> PriorityType {
        match priority {
            10.. => 0,
            0 => 0,
            _ => priority,
        }
    }

    #[inline]
    pub fn as_string(&self) -> String {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn test_todo_into_string() {
        let mut todo = Todo::new("Test".to_string(), 1);
        let _ = todo.set_note("Note".to_string());

        let expected = "[1]>2c924e3088204ee77ba681f72be3444357932fca Test";
        let result: String = (&todo).into();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_try_from_string() {
        let input = "[1]>2c924e3088204ee77ba681f72be3444357932fca Test";
        let expected = Ok(Todo {
            removed_dependency: None,
            schedule: Schedule::new(),
            dependency: Dependency::new_note(
                "2c924e3088204ee77ba681f72be3444357932fca".to_string(),
                "".to_string(),
            ),
            message: "Test".to_string(),
            priority: 1,
            done: false,
        });

        let result: Result<Todo, TodoError> = input.to_string().parse();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_new_todo() {
        let message = "New Todo";
        let priority = 2;

        let todo = Todo::new(message.to_string(), priority);

        assert_eq!(todo.message, message);
        assert_eq!(todo.priority, 2);
        assert_eq!(todo.done, false);
    }

    #[test]
    fn test_dependency_name() {
        let mut todo = Todo::new("Test".to_string(), 1);
        let expected = "900a80c94f076b4ee7006a9747667ccf6878a72b.todo";
        todo.add_todo_dependency();

        let result = &todo.dependency.get_name();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_dependency_type() {
        let mut todo = Todo::new("Test".to_string(), 1);
        todo.add_todo_dependency();

        assert!(todo.dependency.is_list());
    }

    #[test]
    fn test_add_todo() {
        let mut todo = Todo::new("Test".to_string(), 1);
        let expected = "900a80c94f076b4ee7006a9747667ccf6878a72b.todo";
        todo.add_todo_dependency();

        let result = &todo.dependency.get_name();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_add_note_type() {
        let mut todo = Todo::new("Test".to_string(), 1);
        todo.set_note("Note".to_string())
            .expect("Error setting note");

        assert!(todo.dependency.is_note());
    }

    #[test]
    fn test_add_note_name() {
        let mut todo = Todo::new("Test".to_string(), 1);
        todo.set_note("Note".to_string())
            .expect("Error setting note");

        assert_eq!(
            todo.dependency.get_name(),
            "2c924e3088204ee77ba681f72be3444357932fca"
        );
    }

    #[test]
    fn test_remove_note() {
        let mut todo = Todo::new("Test".to_string(), 1);
        todo.set_note("Note".to_string())
            .expect("Error setting note");
        todo.remove_note();

        assert!(todo.dependency.is_none());
    }

    #[test]
    fn test_add_dependency() {
        let mut todo = Todo::new("Test".to_string(), 1);

        todo.add_todo_dependency();

        assert!(todo.dependency.is_list());
    }

    #[test]
    fn test_remove_dependency() {
        let mut todo = Todo::new("Test".to_string(), 1);
        todo.add_todo_dependency();

        todo.remove_dependency();

        assert!(!todo.dependency.is_list());
    }

    #[test]
    fn test_toggle_done() {
        let mut todo = Todo::new("Test".to_string(), 1);

        todo.toggle_done();
        assert_eq!(todo.done(), true);

        todo.toggle_done();
        assert_eq!(todo.done(), false);
    }

    #[test]
    fn test_from_string() {
        let input1 = "[1]>1BE348656D84993A6DF0DB0DECF2E95EF2CF461c.todo Read for exams";
        let todo = Todo::from_str(input1).unwrap();

        let expected = Todo {
            removed_dependency: None,
            schedule: Schedule::new(),
            dependency: Dependency::new_todo_list(
                "1BE348656D84993A6DF0DB0DECF2E95EF2CF461c".to_string(),
            ),
            message: "Read for exams".to_string(),
            priority: 1,
            done: false,
        };
        assert_eq!(todo, expected);
    }

    #[test]
    fn test_daily() {
        let input = "[-2] this one should be daily [D1(2023-09-05)]";
        let todo = Todo::from_str(input).unwrap();
        let expected = Todo {
            removed_dependency: None,
            schedule: Schedule::from("D1(2023-09-05)"),
            dependency: Dependency::default(),
            message: "this one should be daily".to_string(),
            priority: 2,
            done: false,
        };
        assert_eq!(todo, expected);
        let input = "[2] this one should be daily [D1(2023-09-05)]";
        let todo = Todo::from_str(input).unwrap();
        assert_eq!(todo, expected);
    }

    #[test]
    fn test_daily_display() {
        let test = Todo {
            removed_dependency: None,
            dependency: Dependency::default(),
            schedule: Schedule::from("D1()"),
            message: "this one should be daily".to_string(),
            priority: 2,
            done: false,
        };
        let expected = "2. this one should be daily (Daily)";

        assert_eq!(test.display(&DisplayArgs::parse()), expected)
    }

    #[test]
    fn test_weekly() {
        let input = "[-2] this one should be daily [D7(2023-09-05)]";
        let todo = Todo::from_str(input).unwrap();
        let expected = Todo {
            removed_dependency: None,
            dependency: Dependency::default(),
            schedule: Schedule::from("D7(2023-09-05)"),
            message: "this one should be daily".to_string(),
            priority: 2,
            done: false,
        };
        assert_eq!(todo, expected);
        let input = "[2] this one should be daily [D7(2023-09-05)]";
        let todo = Todo::from_str(input).unwrap();
        assert_eq!(todo, expected);
    }
}
