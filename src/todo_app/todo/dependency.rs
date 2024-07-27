use super::Todo;
use super::TodoList;
use std::str::FromStr;
use std::{
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Eq, PartialEq, Clone, Default)]
enum DependencyMode {
    #[default]
    TodoList,
    Note,
}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct Dependency {
    name: String,
    mode: DependencyMode,
    note: String,
    written: bool,
    pub(crate) todo_list: TodoList,
}

impl Dependency {
    #[inline]
    pub fn is_written(&self) -> bool {
        self.written
    }

    #[inline]
    pub fn note(&self) -> Option<&str> {
        if self.is_note() {
            Some(&self.note)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn push(&mut self, todo: Todo) {
        if self.is_list() {
            self.todo_list.push(todo);
        }
    }

    #[inline]
    pub fn new_todo_list(hash: String) -> Self {
        Self {
            name: format!("{}.todo", hash),
            mode: DependencyMode::TodoList,
            ..Default::default()
        }
    }

    #[inline]
    pub fn new_note(hash: String, note: String) -> Self {
        Self {
            note,
            name: hash,
            mode: DependencyMode::Note,
            ..Default::default()
        }
    }

    #[inline]
    pub fn read(&mut self, path: &Path) -> io::Result<()> {
        let file_path = path.join(&self.name);
        let name_todo = format!("{}.todo", self.name);
        match self.mode {
            DependencyMode::Note if path.join(&self.name).is_file() => {
                self.note = std::fs::read_to_string(file_path)?;
            }
            DependencyMode::Note | DependencyMode::TodoList
                // Sometimes calcurse likes to remove the extra .todo from the file name
                // That's why we have the first part of the if statement. c3 itself usually writes
                // the list files to a <sha1>.todo format in notes directory
                if file_path.is_file() || path.join(&name_todo).is_file() => {
                    if self.mode == DependencyMode::Note {
                        self.name = name_todo;
                        self.mode = DependencyMode::TodoList;
                    }
                    self.todo_list = TodoList::read(&path.join(&self.name));
                    self.todo_list.read_dependencies(path);
            }
            _ => {}
        };
        self.written = true;

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
    pub fn todo_list_mut(&mut self) -> Option<&mut TodoList> {
        if self.mode == DependencyMode::TodoList {
            Some(&mut self.todo_list)
        } else {
            None
        }
    }

    #[inline]
    pub fn write(&mut self, path: &Path) -> io::Result<()> {
        match self.mode {
            DependencyMode::TodoList => {
                self.todo_list.write(&path.join(&self.name))?;
            }
            DependencyMode::Note if !self.written => {
                self.write_note(path)?;
            }
            _ => {}
        };
        self.written = true;
        Ok(())
    }

    #[inline]
    fn write_note(&mut self, path: &Path) -> io::Result<()> {
        let mut file = File::create(path.join(&self.name))?;
        write!(file, "{}", self.note)?;
        Ok(())
    }

    #[inline]
    pub fn force_write(&mut self, path: &Path) -> io::Result<()> {
        match self.mode {
            DependencyMode::TodoList => {
                self.todo_list.force_write(&path.join(&self.name))?;
            }
            DependencyMode::Note => {
                self.write_note(path)?;
            }
        };
        self.written = true;
        Ok(())
    }


    #[inline]
    pub fn path(&self, path: &Path) -> Option<PathBuf> {
        path.parent()
            .map(TodoList::append_notes_to_parent)
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
            DependencyMode::Note => ">",
            DependencyMode::TodoList => "-",
        }
    }

    #[inline]
    pub fn remove(&mut self) {
        *self = Self::default();
    }
}

impl From<&Dependency> for String {
    #[inline]
    fn from(dependency: &Dependency) -> String {
        format!(">{}", dependency.name)
    }
}

pub struct EmptyDependency;

impl FromStr for Dependency {
    type Err = EmptyDependency;
    #[inline]
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if !input.is_empty() {
            let name = String::from(input);
            let mode = if input.ends_with(".todo") {
                DependencyMode::TodoList
            } else {
                DependencyMode::Note
            };
            Ok(Self {
                name,
                mode,
                ..Default::default()
            })
        } else {
            Err(EmptyDependency)
        }
    }
}
