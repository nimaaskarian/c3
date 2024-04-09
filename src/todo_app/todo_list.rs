use std::fs::{create_dir_all, read, File};
use std::path::PathBuf;
use std::io::{stdout, BufRead, BufWriter, Write};
use std::io;

use super::{App, Todo, RestrictionFunction};
use crate::DisplayArgs;

#[derive(Debug,PartialEq, Clone, Default)]
pub struct TodoList {
    todos: Vec<Todo>,
}

type Output = Todo;
impl TodoList {
    pub fn new() -> Self{
        TodoList {
            todos: Vec::new(),
        }
    }

    pub fn index(&self, index:usize, restriction: RestrictionFunction) -> &Output {
        if let Some(restriction) = restriction {
            self.todos.iter().filter(|todo| restriction(todo)).nth(index).unwrap()
        } else {
            self.todos.iter().nth(index).unwrap()
        }
    }

    pub fn index_mut(&mut self, index:usize, restriction: RestrictionFunction) -> &mut Output {
        if let Some(restriction) = restriction {
            self.todos.iter_mut().filter(|todo| restriction(todo)).nth(index).unwrap()
        } else {
            self.todos.iter_mut().nth(index).unwrap()
        }
    }

    pub fn todos(&self, restriction: RestrictionFunction) -> Vec<&Todo> {
        if let Some(restriction) = restriction {
            self.todos.iter().filter(|todo| restriction(todo)).collect()
        } else {
            self.todos.iter().collect()
        }
    }

    #[inline]
    pub(super) fn delete_removed_dependent_files(&mut self, filename: &PathBuf) -> io::Result<()> {
        for todo in &mut self.todos {
            todo.delete_removed_dependent_files(filename)?;
        }
        Ok(())
    }

    #[inline]
    pub fn prepend(&mut self, todo:Todo) {
        self.insert(0,todo);
    }

    #[inline]
    pub fn print(&self) -> io::Result<()> {
        let mut stdout_writer = BufWriter::new(stdout());
        self.write_to_buf(&mut stdout_writer)?;
        Ok(())
    }

    pub fn traverse_tree(&self,callback: fn(&mut App, &TodoList, &[usize]), prior_indices: Option<Vec<usize>>, app:&mut App) {
        let prior_indices = prior_indices.unwrap_or(vec![]);
        callback(app, self, prior_indices.as_slice());
        for (i, todo) in self.todos.iter().enumerate() {
            if let Some(todo_list) = todo.dependency.todo_list() {
                let mut prior_indices = prior_indices.clone();
                prior_indices.push(i);
                todo_list.traverse_tree(callback, Some(prior_indices), app);
            }
        }
    }

    pub(super) fn remove_dependency_files(&mut self, filename: &PathBuf) -> io::Result<()> {
        for todo in &mut self.todos {
            todo.delete_dependency_file(filename)?;
        }
        Ok(())
    }

    pub fn read(filename: &PathBuf, read_dependencies: bool, is_root: bool) -> Self{
        let mut todo_array = Self::new();
        if !filename.is_file() {
            return todo_array
        }
        let file_data = read(filename).unwrap();

        for line in file_data.lines() {
            let todo = match Todo::try_from(line.unwrap()) {
                Ok(value) => value,
                Err(..) => continue,
            };
            todo_array.push(todo);
        }
        todo_array.sort();
        if read_dependencies {
            let dependency_path = Self::dependency_parent(filename, is_root);
            let _ = todo_array.read_dependencies(&dependency_path);
        }
        todo_array
    }

    fn read_dependencies(&mut self, path: &PathBuf) -> io::Result<()>{
        for todo in &mut self.todos {
            todo.dependency.read(&path)?;
        }
        Ok(())
    }

    pub fn dependency_parent(filename: &PathBuf, is_root: bool) -> PathBuf {
        if is_root {
            filename.parent().unwrap().to_path_buf().join("notes")
        } else {
            filename.parent().unwrap().to_path_buf()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self{
        TodoList {
            todos: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    fn write_to_buf<W: Write> (&self, writer: &mut BufWriter<W>) -> io::Result<()> {
        for todo in &self.todos {
            writeln!(writer, "{}", todo.as_string())?;
        }
        writer.flush()?;
        Ok(())
    }

    #[inline]
    pub(super) fn write_dependencies(&mut self, filename: &PathBuf) -> io::Result<()> {
        for todo in &mut self.todos {
            todo.dependency.todo_list.write_dependencies(filename)?;
            todo.dependency.write(filename)?;
        }
        Ok(())
    }
    #[inline]
    pub fn write(&mut self, filename: &PathBuf, is_root: bool) -> io::Result<PathBuf> {
        let dependency_path = Self::dependency_parent(filename, is_root);
        create_dir_all(&dependency_path)?;
        let file = File::create(filename)?;
        let mut writer = BufWriter::new(file);
        self.write_to_buf(&mut writer)?;
        Ok(dependency_path)
    }


    pub fn iter(&self) -> std::slice::Iter<Todo> {
        self.todos.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<Todo> {
        self.todos.iter_mut()
    }

    pub fn messages(&self, restriction: RestrictionFunction) -> Vec<String> {
        self.todos(restriction).iter().map(|todo| todo.message.clone()).collect()
    }

    pub fn display(&self, args: &DisplayArgs, restriction: RestrictionFunction) -> Vec<String> {
        self.todos(restriction).iter().map(|todo| todo.display(&args)).collect()
    }

    pub fn len(&self, restriction: RestrictionFunction) -> usize {
        self.todos(restriction).len()
    }

    pub fn is_empty(&self, restriction: RestrictionFunction) -> bool {
        self.todos(restriction).is_empty()
    }

    pub fn remove(&mut self, index:usize) -> Todo{
        self.todos.remove(index).clone()
    }

    pub fn push(&mut self,item:Todo) {
        self.todos.push(item)
    }

    fn insert(&mut self,index:usize, item:Todo) {
        self.todos.insert(index,item)
    }

    #[inline(always)]
    fn reorder_low_high(&self, index:usize) -> (usize, usize){
        let priority = self.todos[index].comparison_priority();
        if index+1 < self.todos.len() && priority > self.todos[index+1].comparison_priority() {
            (index+1, self.todos.len()-1)
        } else {
            (0, index)
        }
    }

    #[inline(always)]
    pub fn reorder_last(&mut self) -> usize {
        self.reorder(self.todos.len()-1)
    }


    pub fn reorder(&mut self, index:usize) -> usize {
        let priority = self.todos[index].comparison_priority();

        if priority < self.todos[0].comparison_priority() {
            return self.move_index(index, 0, 1)
        }

        let (low, high) = self.reorder_low_high(index);
        for middle in low..high {
            if priority < self.todos[middle+1].comparison_priority() &&
            priority >= self.todos[middle].comparison_priority() {
                return self.move_index(index, middle, 0);
            }
        }
        self.move_index(index, high, 0)
    }

    #[inline(always)]
    fn move_index(&mut self, from: usize, to: usize, shift:usize) -> usize{

        let mut i = from;
        if from < to
        {
            for j in from..to {
                self.todos.swap(j, j+1);
                i = j+1;
            }
        } else {
            for j in (to+1-shift..from).rev() {
                self.todos.swap(j, j+1);
                i = j;
            }

        }
        i
    }

    pub fn append_list(&mut self, mut todo_list: TodoList) {
        self.todos.append(&mut todo_list.todos);
        self.sort();
    }

    #[inline(always)]
    pub fn sort (&mut self) {
        // , ascending:Option<bool>
        // let ascending = ascending.unwrap_or(false);
        self.todos.sort_by(|a, b| a.comparison_priority().cmp(&b.comparison_priority()));
    }
}
