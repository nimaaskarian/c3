use std::fs::File;
use std::path::PathBuf;
use std::ops::{Index, IndexMut};
use std::io::{BufWriter, Write};
use std::io;
use std::fs::{read_to_string};
mod todo;
use todo::Todo;

pub struct TodoArray {
    todos: Vec<Todo>
}

impl Index<usize> for TodoArray {
    type Output = Todo;
    fn index(&self, index:usize) -> &Self::Output {
        &self.todos[index]
    }
}

impl IndexMut<usize> for TodoArray {
    // type Output = Todo;
    fn index_mut(&mut self, index:usize) -> &mut Todo {
        &mut self.todos[index]
    }
}

pub enum TodoArrayError {
    IndexNotFound
}

impl TodoArray {
    fn new() -> Self{
        TodoArray {
            todos: Vec::new()
        }
    }
    fn push(&mut self,item:Todo) {
        self.todos.push(item)
    }

    #[inline(always)]
    fn reorder_low_high(&self, index:usize) -> (usize, usize){
        let priority = self.todos[index].comparison_priority();
        if index+1 < self.todos.len() && priority > self.todos[index+1].comparison_priority() {
            (index+1, self.todos.len()-1)
        } else {
            (0, index-1)
        }
    }

    pub fn reorder(&mut self, index:usize) -> Result<(), TodoArrayError> {
        if index > self.todos.len() {
            return Err(TodoArrayError::IndexNotFound);
        }
        let priority = self.todos[index].comparison_priority();
        if priority < self.todos[0].comparison_priority() {
            self.move_index(index, 0, 1)
        }
        let (mut low, mut high) = self.reorder_low_high(index);

        while low < high {
            let middle = (low + high) / 2;
            if priority < self.todos[middle + 1].comparison_priority()
                && priority >= self.todos[middle].comparison_priority()
            {
                self.move_index(index, middle, 0);
                return Ok(());
            }

            if priority < self.todos[middle].comparison_priority() {
                high = middle - 1;
            } else {
                low = middle + 1;
            }
        }
        // If isn't first and not in the middle, then its the last one
        self.move_index(index, self.todos.len()-1, 0);
        Ok(())
    }

    #[inline(always)]
    fn move_index(&mut self, from: usize, to: usize, shift:usize) {

        if from < to
        {
            for i in from..to {
                self.todos.swap(i, i+1);
            }
        } else {
            for i in (to+1-shift..from).rev() {
                self.todos.swap(i, i+1);
            }

        }
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
        for todo in &self.todos {
            println!("{}", todo.as_string());
        }
    }

    #[inline(always)]
    pub fn sort (&mut self) {
        // , ascending:Option<bool>
        // let ascending = ascending.unwrap_or(false);
        self.todos.sort_by(|a, b| a.comparison_priority().cmp(&b.comparison_priority()));
    }
}

pub struct TodoList {
    pub undone: TodoArray,
    pub done: TodoArray,
}

impl TodoList {
    pub fn read (filename: &PathBuf) -> Self{
        let mut undone = TodoArray::new();
        let mut done = TodoArray::new();
        if !filename.exists() {
            return TodoList {
                done,
                undone
            }
        }
        for line in read_to_string(filename).unwrap().lines() {
            let todo = match Todo::try_from(line) {
                Ok(value) => value,
                Err(..) => continue,
            };
            if todo.done {
                done.push(todo);
            } else {
                undone.push(todo);
            }
        }
        TodoList {
            done, 
            undone
        }
    }

    pub fn write (&self, filename: &str) -> io::Result<()> {
        let file = File::create(filename)?;
        let mut writer = BufWriter::new(file);
        let todos = [&self.done.todos, &self.undone.todos];

        for todo in todos.iter().flat_map(|v| v.iter()) {
            writeln!(writer, "{}", todo.as_string())?;
        }
        writer.flush()?;
        Ok(())
    }
}
