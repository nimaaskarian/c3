use scanf::sscanf;
use std::{io::{self, Write}, path::PathBuf, fs::File};
use super::TodoList;
use super::Todo;

#[derive(Debug, PartialEq, Clone, Default)]
pub enum Mode {
    #[default]
    None,
    TodoList,
    Note,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct TodoDependency {
    name: String,
    mode: Mode,
    note: String,
    pub(crate) todo_list: TodoList,
}

impl TodoDependency {
    pub fn default() -> Self {
        TodoDependency {
            mode: Mode::None,
            name: String::new(),
            note: String::new(),
            todo_list: TodoList::new(),
        }
    }

    pub fn new(name: String, mode: Mode) -> Self {
        TodoDependency {
            mode,
            name,
            note: String::new(),
            todo_list: TodoList::new(),
        }
    }

    pub fn note(&self) -> Option<&String> {
        if self.mode == Mode::Note {
            Some(&self.note)
        } else {
            None
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn push(&mut self, todo:Todo) {
        if self.mode == Mode::TodoList {
            self.todo_list.push(todo);
        }
    }

    pub fn new_todo_list(hash: String) -> Self {
        TodoDependency::new(format!("{}.todo", hash), Mode::TodoList)
    }

    pub fn new_note(hash: String, note: String) -> Self {
        TodoDependency::new(format!("{}", hash), Mode::Note)
    }

    pub fn read(&mut self, path: &PathBuf)  -> io::Result<()> {
        match self.mode {
            Mode::Note => {
                self.note = std::fs::read_to_string(path.join(&self.name))?;
            }
            Mode::TodoList => {
                self.todo_list = TodoList::read(&path.join(&self.name), true, false);
            }
            Mode::None => {}
        };

        Ok(())
    }

    pub fn todo_list(&self) -> Option<&TodoList> {
        if self.mode == Mode::TodoList {
            Some(&self.todo_list)
        } else {
            None
        }
    }

    pub fn write(&mut self, path: &PathBuf) -> io::Result<()> {
        let name = self.name.clone();
        match self.mode.clone() {
            Mode::TodoList => {
                self.todo_list.write(&path.join(&self.name), false)?;
            }
            Mode::Note => {
                let mut file = File::create(path.join(name))?;
                write!(file, "{}", self.note);
            }
            _ => {}
        };
        Ok(())
    }

    pub fn path(&self ,path: &PathBuf) -> Option<PathBuf>{
        match path.parent() {
            Some(path) => Some(TodoList::dependency_parent(&path.to_path_buf(), false)),
            None => None,
        }
    }

    pub fn is_none(&self) -> bool {
        self.mode == Mode::None
    }

    pub fn is_note(&self) -> bool {
        self.mode == Mode::Note
    }

    pub fn is_list(&self) -> bool {
        self.mode == Mode::TodoList
    }

    pub fn display<'a>(&self) -> &'a str {
        match self.mode {
            Mode::None => ".",
            Mode::Note => ">",
            Mode::TodoList => "-",
        }
    }

    pub fn remove(&mut self) -> Option<String> {
        let name = match self.mode {
            Mode::None => None,
            _ => Some(self.name.clone()),
        };
        *self = Self::default();

        return name
    }
}


impl Into<String> for &TodoDependency {
    fn into (self) -> String {
        match self.mode {
            Mode::None => String::new(),
            _ => format!(">{}", self.name),
        }
    }
}

impl From<&str> for TodoDependency {
    fn from (input: &str) -> TodoDependency {
        let mut name = String::new();
        let mode: Mode;

        match input {
            _ if sscanf!(input, "{}.todo", name).is_ok() => {
                mode = Mode::TodoList;
                name = format!("{name}.todo");
            }
            _ if sscanf!(input, "{}", name).is_ok() && !name.is_empty() => {
                mode = Mode::Note;
            }
            _ => {
                mode = Mode::None;
            }
        };

        TodoDependency::new(name, mode)
    }
}

