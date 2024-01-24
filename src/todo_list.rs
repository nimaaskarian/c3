use std::fs::{File, create_dir_all};
use std::path::PathBuf;
use std::ops::{Index, IndexMut};
use std::io::{BufWriter, Write, stdout};
use std::io;
use std::fs::read_to_string;
pub mod todo;
use todo::Todo;

#[derive(Debug,PartialEq, Clone, Default)]
pub struct TodoArray {
    pub todos: Vec<Todo>
}

impl Index<usize> for TodoArray {
    type Output = Todo;
    fn index(&self, index:usize) -> &Self::Output {
        &self.todos[index]
    }
}

impl IndexMut<usize> for TodoArray {
    fn index_mut(&mut self, index:usize) -> &mut Todo {
        &mut self.todos[index]
    }
}

pub enum TodoArrayError {
    IndexNotFound
}

impl Index<usize> for TodoList {
    type Output = Todo;
    fn index(&self, index:usize) -> &Self::Output {
        let size = self.undone.len();
        if index < size {
            &self.undone[index]
        } else {
            &self.done[index-size]
        }
    }
}

impl IndexMut<usize> for TodoList {
    fn index_mut(&mut self, index:usize) -> &mut Todo {
        let size = self.undone.len();
        if index < size {
            &mut self.undone[index]
        } else {
            &mut self.done[index-size]
        }
    }
}

impl TodoArray {
    pub fn new() -> Self{
        TodoArray {
            todos: Vec::new()
        }
    }

    pub fn messages(&self) -> Vec<String> {
        self.todos.iter().map(|todo| todo.message.clone()).collect()
    }

    pub fn display(&self, show_done: Option<bool>) -> Vec<String> {
        self.todos.iter().map(|todo| todo.display(show_done)).collect()
    }

    pub fn len(&self) -> usize {
        self.todos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.todos.is_empty()
    }

    pub fn remove(&mut self, index:usize) -> Todo{
        self.todos.remove(index)
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
        return self.move_index(index, high, 0);
    }

    #[inline(always)]
    fn move_index(&mut self, from: usize, to: usize, shift:usize) -> usize{

        let mut j = from;
        if from < to
        {
            for i in from..to {
                self.todos.swap(i, i+1);
                j = i+1;
            }
        } else {
            for i in (to+1-shift..from).rev() {
                self.todos.swap(i, i+1);
                j = i;
            }

        }
        return j;
        // if to == from {
        //     return;
        // }

        // let tmp = std::mem::replace(&mut self.todos[from], Default::default());

        // if to < from {
        //     self.todos.insert(to, tmp);
        //     self.todos.remove(from + 1);
        // } else {
        //     self.todos.insert(to + 1, tmp);
        //     self.todos.remove(from);
        // }
    }

    pub fn print (&self) {
        let mut i = 1;
        for todo in &self.todos {
            println!("{} - {}", i,todo.as_string());
            i+=1;
        }
    }

    #[inline(always)]
    pub fn sort (&mut self) {
        // , ascending:Option<bool>
        // let ascending = ascending.unwrap_or(false);
        self.todos.sort_by(|a, b| a.comparison_priority().cmp(&b.comparison_priority()));
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct TodoList {
    pub undone: TodoArray,
    pub done: TodoArray,
}

impl TodoList {
    pub fn new () -> Self {
        let undone = TodoArray::new();
        let done = TodoArray::new();

        TodoList {
            done,
            undone
        }
    }

    pub fn remove(&mut self, index: usize) -> Todo {
        let size = self.undone.len();

        if index < size {
            self.undone.remove(index)
        } else {
            self.done.remove(index - size)
        }
        
    }

    pub fn reorder(&mut self, index: usize) -> usize {
        let size = self.undone.len();

        if index < size {
            self.undone.reorder(index)
        } else {
            self.done.reorder(index - size) + size
        }
    }

    pub fn is_empty(&self) -> bool {
        self.undone.is_empty() && self.done.is_empty()
    }

    pub fn len(&self) -> usize {
        self.undone.len() + self.done.len()
    }

    pub fn display(&self, show_done: bool) -> Vec<String> {
        let arg = Some(show_done);
        let mut display_list = self.undone.display(arg);

        if show_done {
            display_list.extend(self.done.display(arg));
        }
        display_list
    }

    pub fn read (filename: &PathBuf, read_dependencies: bool, is_root: bool) -> Self{
        let dependency_path = if is_root {
            filename.parent().unwrap().to_path_buf().join("notes")
        } else {
            filename.parent().unwrap().to_path_buf()
        };
        let mut todo_list = Self::new();
        if !filename.is_file() {
            return todo_list
        }
        for line in read_to_string(filename).unwrap().lines() {
            let todo = match Todo::try_from(line) {
                Ok(value) => value,
                Err(..) => continue,
            };
            todo_list.push(todo)
        }
        todo_list.undone.sort();
        todo_list.done.sort();
        if read_dependencies {
            todo_list.read_dependencies(&dependency_path)
        }
        return todo_list
    }

    pub fn read_dependencies(&mut self, path: &PathBuf) {
        let mut todos = [&mut self.undone.todos, &mut self.done.todos];

        for todo in todos.iter_mut().flat_map(|v| v.iter_mut()) {
            todo.read_dependencies(&path);
        }
    }

    #[inline]
    pub fn push(&mut self, todo:Todo) {
        if todo.done() {
            self.done.push(todo);
        } else {
            self.undone.push(todo);
        }
    }

    #[inline]
    pub fn prepend(&mut self, todo:Todo) {
        self.undone.insert(0,todo);
    }

    #[inline]
    pub fn fix_undone(&mut self) {
        for index in 0..self.undone.todos.len() {
            if self.undone.todos[index].done() {
                self.done.push(self.undone.todos.remove(index));
            }
            if index+1 >= self.undone.todos.len() {
                break;
            }
        }
    }
    
    pub fn fix_done(&mut self) {
        for index in 0..self.done.todos.len() {
            if !self.done.todos[index].done() {
                self.undone.push(self.done.todos.remove(index));
            }
            if index+1 >= self.done.todos.len() {
                break;
            }
        }

    }

    #[inline]
    fn write_to_buf<W: Write> (&self, writer: &mut BufWriter<W>) -> io::Result<()> {
        let todos = [&self.undone.todos, &self.done.todos];

        for todo in todos.iter().flat_map(|v| v.iter()) {
            writeln!(writer, "{}", todo.as_string())?;
        }
        writer.flush()?;
        Ok(())
    }

    #[inline]
    fn handle_dependent_files(&mut self, path: &PathBuf) -> io::Result<()> {
        let mut todos = [&mut self.undone.todos, &mut self.done.todos];
        for todo in todos.iter_mut().flat_map(|v| v.iter_mut()) {
            todo.dependency_write(path)?;
            todo.remove_dependent_files()?;
        }
        Ok(())
    }

    #[inline]
    pub fn write(&mut self, filename: &PathBuf, is_root: bool) -> io::Result<()> {
        let dependency_path = if is_root {
            filename.parent().unwrap().to_path_buf().join("notes")
        } else {
            filename.parent().unwrap().to_path_buf()
        };
        create_dir_all(&dependency_path)?;
        let file = File::create(filename)?;
        let mut writer = BufWriter::new(file);
        self.write_to_buf(&mut writer)?;
        self.handle_dependent_files(&dependency_path)?;
        Ok(())
    }

    #[inline]
    pub fn print(&self) -> io::Result<()> {
        let mut stdout_writer = BufWriter::new(stdout());
        self.write_to_buf(&mut stdout_writer)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use std::fs::{self, remove_file};

    use super::*;

    fn get_todo_list() -> TodoList {
        let path = PathBuf::from("tests/TODO_LIST");
        TodoList::read(&path, true, true)
    }

    #[test]
    fn test_todolist_read_undone() {
        let todo_list = get_todo_list();
        let expected_undone = vec![Todo::new("this todo has prio 1".to_string(), 1,false, None)
            ,Todo::new("this one has prio 2".to_string(), 2, false ,None)];
        assert_eq!(expected_undone, todo_list.undone.todos);
    }

    #[test]
    fn test_todolist_read_done() {
        let todo_list = get_todo_list();
        let expected_done = vec![Todo::new("this one is 2 and done".to_string(), 2, true, None),Todo::new("this one is 0 and done".to_string(), 0, true, None)];
        assert_eq!(expected_done, todo_list.done.todos);
    }

    #[test]
    fn test_len() {
        let todo_list = get_todo_list();
        assert_eq!(todo_list.len(), 4);
    }

    #[test]
    fn test_write() {
        let mut todo_list = get_todo_list();
        let path = PathBuf::from("tests/tmplist");
        todo_list.write(&path, true);

        let contents = fs::read_to_string(&path).expect("Reading file failed :(");
        let expected = "[1] this todo has prio 1
[2] this one has prio 2
[-2] this one is 2 and done
[-0] this one is 0 and done
";

        remove_file(path);
        assert_eq!(contents, expected)
    }

    #[test]
    fn test_push() {
        let mut todo_list = get_todo_list();
        let path = PathBuf::from("tests/tmplist");
        todo_list.push(Todo::default("Show me your warface".to_string(), 0));
        todo_list.write(&path, true);

        let contents = fs::read_to_string(&path).expect("Reading file failed :(");
        let expected = "[1] this todo has prio 1
[2] this one has prio 2
[0] Show me your warface
[-2] this one is 2 and done
[-0] this one is 0 and done
";

        remove_file(path);
        assert_eq!(contents, expected);
    }

    #[test]
    fn test_initially_sorted() {
        let todo_list = get_todo_list();
        let mut sorted_list = todo_list.clone();
        sorted_list.undone.sort();
        sorted_list.done.sort();

        assert_eq!(todo_list, sorted_list)
    }
}
