use scanf::sscanf;
use std::{io::{self, Write}, path::PathBuf, fs::File};
use super::TodoList;
use super::Todo;

#[derive(Debug, PartialEq, Clone, Default)]
enum DependencyMode {
    #[default]
    None,
    TodoList,
    Note,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Dependency {
    name: String,
    mode: DependencyMode,
    note: String,
    pub(crate) todo_list: TodoList,
}

impl Dependency {
    #[inline]
    pub fn default() -> Self {
        Dependency {
            mode: DependencyMode::None,
            name: String::new(),
            note: String::new(),
            todo_list: TodoList::new(),
        }
    }

    #[inline]
    fn new(name: String, mode: DependencyMode) -> Self {
        Dependency {
            mode,
            name,
            note: String::new(),
            todo_list: TodoList::new(),
        }
    }

    #[inline]
    pub fn note(&self) -> Option<&String> {
        if self.is_note() {
            Some(&self.note)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    #[inline]
    pub fn push(&mut self, todo:Todo) {
        if self.is_list() {
            self.todo_list.push(todo);
        }
    }

    #[inline]
    pub fn new_todo_list(hash: String) -> Self {
        Dependency::new(format!("{}.todo", hash), DependencyMode::TodoList)
    }

    #[inline]
    pub fn new_note(hash: String, note: String) -> Self {
        let mut dependency = Dependency::new(format!("{}", hash), DependencyMode::Note);
        dependency.note = note;

        dependency
    }

    #[inline]
    pub fn read(&mut self, path: &PathBuf)  -> io::Result<()> {
        match self.mode {
            DependencyMode::Note => {
                self.note = std::fs::read_to_string(path.join(&self.name))?;
            }
            DependencyMode::TodoList => {
                self.todo_list = TodoList::read(&path.join(&self.name), true, false);
            }
            DependencyMode::None => {}
        };

        Ok(())
    }

    #[inline]
    pub fn todo_list(&self) -> Option<&TodoList> {
        if self.mode == DependencyMode::TodoList {
            Some(&self.todo_list)
        } else {
            None
        }
    }

    #[inline]
    pub fn write(&mut self, path: &PathBuf) -> io::Result<()> {
        let name = self.name.clone();
        match self.mode.clone() {
            DependencyMode::TodoList => {
                self.todo_list.write(&path.join(&self.name), false)?;
            }
            DependencyMode::Note => {
                let mut file = File::create(path.join(name))?;
                write!(file, "{}", self.note)?;
            }
            _ => {}
        };
        Ok(())
    }

    #[inline]
    pub fn path(&self ,path: &PathBuf) -> Option<PathBuf>{
        match path.parent() {
            Some(path) => Some(TodoList::dependency_parent(&path.to_path_buf(), false)),
            None => None,
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.mode == DependencyMode::None
    }

    #[inline]
    pub fn is_note(&self) -> bool {
        self.mode == DependencyMode::Note
    }

    #[inline]
    pub fn is_list(&self) -> bool {
        self.mode == DependencyMode::TodoList
    }

    #[inline]
    pub fn display<'a>(&self) -> &'a str {
        match self.mode {
            DependencyMode::None => ".",
            DependencyMode::Note => ">",
            DependencyMode::TodoList => "-",
        }
    }

    #[inline]
    pub fn remove(&mut self) -> Option<String> {
        let name = match self.mode {
            DependencyMode::None => None,
            _ => Some(self.name.clone()),
        };
        *self = Self::default();

        return name
    }
}


impl Into<String> for &Dependency {
    #[inline]
    fn into (self) -> String {
        match self.mode {
            DependencyMode::None => String::new(),
            _ => format!(">{}", self.name),
        }
    }
}

impl From<&str> for Dependency {
    #[inline]
    fn from (input: &str) -> Dependency {
        let mut name = String::new();
        let mode: DependencyMode;

        match input {
            _ if sscanf!(input, "{}.todo", name).is_ok() => {
                mode = DependencyMode::TodoList;
                name = format!("{name}.todo");
            }
            _ if sscanf!(input, "{}", name).is_ok() && !name.is_empty() => {
                mode = DependencyMode::Note;
            }
            _ => {
                mode = DependencyMode::None;
            }
        };

        Dependency::new(name, mode)
    }
}

