use std::fmt;
use std::path::Path;
// vim:fileencoding=utf-8:foldmethod=marker
//std{{{
use std::str::{self, FromStr};
use std::{fs::remove_file, io};
//}}}
// mod{{{
mod dependency;
mod note;
pub mod schedule;
use super::TodoList;
use crate::{DisplayArgs, TodoDisplay};
use dependency::Dependency;
use note::{open_note_temp_editor, sha1};
use schedule::Schedule;
//}}}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct Todo {
    pub message: String,
    priority: u8,
    pub dependency: Option<Dependency>,
    removed_dependency: Option<Dependency>,
    last_schedule: Option<Schedule>,
    done: bool,
    pub schedule: Option<Schedule>,
}

impl fmt::Display for Todo {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shcedule_str = self
            .schedule
            .as_ref()
            .map(|schedule| schedule.display())
            .unwrap_or_default();
        let note_string = self.dependency.as_ref().map_or(".", |dep| dep.display());
        let Todo {
            priority, message, ..
        } = self;

        write!(f, "{priority}{note_string} {message}{shcedule_str}")
    }
}

impl TodoDisplay for Todo {
    #[inline(always)]
    fn display_with_args(&self, args: &DisplayArgs) -> String {
        let done_string = if args.show_done {
            if self.done() {
                args.done_string.as_str()
            } else {
                args.undone_string.as_str()
            }
        } else {
            ""
        };
        format!("{done_string}{self}")
    }
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
        let dep_str: String = todo
            .dependency
            .as_ref()
            .map(|x| x.into())
            .unwrap_or_default();

        let schedule_str: String = todo.schedule.as_ref().map(|x| x.into()).unwrap_or_default();

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
                        priority = c.to_digit(10).unwrap() as u8;
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
            let schedule: Option<Schedule> = schedule_string.parse().ok();
            let dependency = dependency_string.parse().ok();

            if let Some(schedule) = schedule.as_ref() {
                done =  schedule.date_should_be_done();
            }
            Ok(Todo {
                dependency,
                schedule,
                message,
                priority,
                done,
                ..Default::default()
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
    pub fn priority(&self) -> u8 {
        self.priority
    }
    #[inline]
    pub fn new(message: String, priority: u8) -> Self {
        Todo {
            message,
            priority: Self::fixed_priority(priority),
            ..Default::default()
        }
    }

    pub fn toggle_schedule(&mut self) -> bool {
        if self.schedule.is_none() && self.last_schedule.is_none() {
            return false;
        }
        if self.schedule.is_some() {
            self.last_schedule = std::mem::take(&mut self.schedule);
        } else {
            self.schedule = std::mem::take(&mut self.last_schedule);
        }
        true
    }

    pub fn abandonment_coefficient(&self) -> f64 {
        self.schedule.as_ref().map_or(1., |sch| {
            if sch.is_reminder() {
                return 1.;
            }
            let days_diff = sch.days_diff();
            if days_diff == 0 {
                1.
            } else {
                sch.days_diff() as f64 / sch.days() as f64
            }
        })
    }

    #[inline]
    pub fn is_note(&self) -> bool {
        self.dependency.as_ref().map_or(false, |dep| dep.is_note())
    }

    #[inline]
    pub fn dependencies(&self) -> Option<&TodoList> {
        self.dependency.as_ref()?.todo_list()
    }

    #[inline]
    pub fn no_dependency(&self) -> bool {
        self.dependency.is_none()
    }

    #[inline]
    pub fn add_todo_dependency(&mut self) {
        if self.dependency.is_none() {
            self.dependency = Some(Dependency::new_todo_list(self.hash()));
        }
    }

    #[inline]
    pub fn delete_dependency_file(&mut self, path: &Path) -> io::Result<()> {
        if let Some(dependency) = &mut self.dependency {
            dependency.todo_list.remove_dependency_files(path)?;
            let _ = remove_file(path.join(dependency.get_name()));
        }
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
    pub fn remove_dependency(&mut self) {
        if let Some(dependency) = self.dependency.as_mut() {
            if dependency.is_written() {
                self.removed_dependency = Some(std::mem::take(dependency));
            }
        }
        self.dependency = None;
    }

    #[inline]
    pub fn set_note(&mut self, note: String) -> io::Result<()> {
        self.dependency = Some(Dependency::new_note(sha1(&note), note));
        Ok(())
    }

    #[inline]
    pub fn edit_note(&mut self) -> io::Result<bool> {
        let note = self.dependency.as_ref().and_then(|dep| dep.note());
        let new_note = open_note_temp_editor(note)?;
        if !new_note.is_empty() && note.map_or(true, |note| note != new_note) {
            self.set_note(new_note)?;
            Ok(true)
        } else {
            Ok(false)
        }
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
        if self.toggle_schedule() {
            if let Some(schedule) = self.schedule.as_mut() {
                schedule.set_day(1);
            }
        } else {
            self.schedule = Some(Schedule::new(1))
        }
    }

    #[inline]
    pub fn toggle_weekly(&mut self) {
        if self.toggle_schedule() {
            if let Some(schedule) = self.schedule.as_mut() {
                schedule.set_day(7);
            }
        } else {
            self.schedule = Some(Schedule::new(7))
        }
    }

    #[inline]
    pub fn enable_day(&mut self, day: i64) {
        self.schedule = Schedule::default().into();
        self.schedule.as_mut().unwrap().set_day(day);
    }

    #[inline]
    pub fn set_done(&mut self, done: bool) {
        if let Some(schedule) = self.schedule.as_mut() {
            schedule.set_current_date();
        }
        self.done = done;
    }

    #[inline(always)]
    fn standardize_priority(priority: u8) -> u8 {
        match priority {
            0 => 10,
            any => any,
        }
    }

    #[inline(always)]
    fn standardized_priority(&self) -> u8 {
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
    pub fn set_priority(&mut self, priority: u8) {
        self.priority = priority;
        self.fix_priority();
    }

    #[inline]
    fn fix_priority(&mut self) {
        self.priority = Todo::fixed_priority(self.priority)
    }

    #[inline(always)]
    fn cmp_value(&self) -> u8 {
        let mut priority = self.standardized_priority() * 2;
        if self.schedule.as_ref().map_or(false, Schedule::is_reminder) {
            priority -= 1;
        }
        if self.done() {
            priority += Self::standardize_priority(0) * 2;
        }

        priority
    }

    #[inline]
    fn fixed_priority(priority: u8) -> u8 {
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
            last_schedule: None,
            removed_dependency: None,
            schedule: None,
            dependency: Some(Dependency::new_note(
                "2c924e3088204ee77ba681f72be3444357932fca".to_string(),
                "".to_string(),
            )),
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

        let result = todo.dependency.unwrap().get_name().to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_dependency_type() {
        let mut todo = Todo::new("Test".to_string(), 1);
        todo.add_todo_dependency();

        assert!(todo.dependency.unwrap().is_list());
    }

    #[test]
    fn test_add_todo() {
        let mut todo = Todo::new("Test".to_string(), 1);
        let expected = "900a80c94f076b4ee7006a9747667ccf6878a72b.todo";
        todo.add_todo_dependency();

        let result = todo.dependency.unwrap().get_name().to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_add_note_type() {
        let mut todo = Todo::new("Test".to_string(), 1);
        todo.set_note("Note".to_string())
            .expect("Error setting note");

        assert!(todo.dependency.unwrap().is_note());
    }

    #[test]
    fn test_add_note_name() {
        let mut todo = Todo::new("Test".to_string(), 1);
        todo.set_note("Note".to_string())
            .expect("Error setting note");

        assert_eq!(
            todo.dependency.unwrap().get_name(),
            "2c924e3088204ee77ba681f72be3444357932fca"
        );
    }

    #[test]
    fn test_add_dependency() {
        let mut todo = Todo::new("Test".to_string(), 1);

        todo.add_todo_dependency();

        assert!(todo.dependency.unwrap().is_list());
    }

    #[test]
    fn test_remove_dependency() {
        let mut todo = Todo::new("Test".to_string(), 1);
        todo.add_todo_dependency();

        todo.remove_dependency();

        assert!(todo.dependency.is_none());
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
            dependency: Some(Dependency::new_todo_list(
                "1BE348656D84993A6DF0DB0DECF2E95EF2CF461c".to_string(),
            )),
            message: "Read for exams".to_string(),
            priority: 1,
            ..Default::default()
        };
        assert_eq!(todo, expected);
    }

    #[test]
    fn test_daily() {
        let input = "[-2] this one should be daily [D1(2023-09-05)]";
        let todo = Todo::from_str(input).unwrap();
        let expected = Todo {
            schedule: "D1(2023-09-05)".parse().ok(),
            message: "this one should be daily".to_string(),
            priority: 2,
            ..Default::default()
        };
        assert_eq!(todo, expected);
        let input = "[2] this one should be daily [D1(2023-09-05)]";
        let todo = Todo::from_str(input).unwrap();
        assert_eq!(todo, expected);
    }

    #[test]
    fn test_daily_display() {
        let test = Todo {
            schedule: "D1()".parse().ok(),
            message: "this one should be daily".to_string(),
            priority: 2,
            ..Default::default()
        };
        let expected = "2. this one should be daily (Daily)";

        assert_eq!(test.display_with_args(&DisplayArgs::parse()), expected)
    }

    #[test]
    fn test_weekly() {
        let input = "[-2] this one should be daily [D7(2023-09-05)]";
        let todo = Todo::from_str(input).unwrap();
        let expected = Todo {
            schedule: "D7(2023-09-05)".parse().ok(),
            message: "this one should be daily".to_string(),
            priority: 2,
            ..Default::default()
        };
        assert_eq!(todo, expected);
        let input = "[2] this one should be daily [D7(2023-09-05)]";
        let todo = Todo::from_str(input).unwrap();
        assert_eq!(todo, expected);
    }
}
