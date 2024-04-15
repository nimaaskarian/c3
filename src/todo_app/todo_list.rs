use std::fs::{create_dir_all, read, File};
use std::path::PathBuf;
use std::io::{stdout, BufRead, BufWriter, Write};
use std::io;

use super::{App, Todo, Restriction};
use crate::DisplayArgs;

#[derive(Debug,PartialEq, Clone, Default)]
pub struct TodoList {
    pub todos: Vec<Todo>,
}

type Output = Todo;
impl TodoList {
    pub fn new() -> Self{
        TodoList {
            todos: Vec::new(),
        }
    }

    pub fn index(&self, index:usize, restriction: Restriction) -> &Output {
        if let Some(restriction) = restriction {
            self.todos.iter().filter(|todo| restriction(todo)).nth(index).unwrap()
        } else {
            self.todos.iter().nth(index).unwrap()
        }
    }

    pub fn index_mut(&mut self, index:usize, restriction: Restriction) -> &mut Output {
        if let Some(restriction) = restriction {
            self.todos.iter_mut().filter(|todo| restriction(todo)).nth(index).unwrap()
        } else {
            self.todos.iter_mut().nth(index).unwrap()
        }
    }

    pub fn todos(&self, restriction: Restriction) -> Vec<&Todo> {
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

    #[inline(always)]
    pub (super) fn set_todos(&mut self, todos:Vec<Todo>) {
        self.todos = todos
    }

    pub fn iter(&self) -> std::slice::Iter<Todo> {
        self.todos.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<Todo> {
        self.todos.iter_mut()
    }

    pub fn messages(&self, restriction: Restriction) -> Vec<String> {
        self.todos(restriction).iter().map(|todo| todo.message.clone()).collect()
    }

    pub fn display(&self, args: &DisplayArgs, restriction: Restriction) -> Vec<String> {
        self.todos(restriction).iter().map(|todo| todo.display(&args)).collect()
    }

    pub fn len(&self, restriction: Restriction) -> usize {
        self.todos(restriction).len()
    }

    pub fn is_empty(&self, restriction: Restriction) -> bool {
        self.todos(restriction).is_empty()
    }

    pub fn remove(&mut self, index:usize, restriction: Restriction) {
        let mut binding = self.todos(restriction);
        let filtered:Vec<_> = binding.iter_mut().collect();
        self.todos = self.todos.iter().filter(|x| &x != &filtered[index]).cloned().collect();
    }

    pub fn true_position_in_list(&mut self, index:usize, restriction: Restriction) -> usize {
        let mut binding = self.todos(restriction);
        let filtered:Vec<_> = binding.iter_mut().collect();
        self.todos.iter().position(|x| &x == filtered[index]).unwrap()
    }

    pub fn cut(&mut self, index:usize, restriction: Restriction) -> Todo{
        let index_in_vec = self.true_position_in_list(index, restriction);
        self.todos.remove(index_in_vec)
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
        if from < to {
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
        self.todos.sort_by(|a, b| a.comparison_priority().cmp(&b.comparison_priority()));
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, remove_dir_all, remove_file};

    use super::*;

    fn get_todo_list() -> TodoList {
        let path = PathBuf::from("tests/TODO_LIST");
        TodoList::read(&path, true, true)
    }

    #[test]
    fn test_todolist_read_undone() {
        let todo_list = get_todo_list();
        let expected_undone = vec![Todo::written("this todo has prio 1".to_string(), 1, false)
            ,Todo::written("this one has prio 2".to_string(), 2, false)];

        assert_eq!(expected_undone, todo_list.todos.iter().filter(|todo| !todo.done()).cloned().collect::<Vec<Todo>>());
    }

    #[test]
    fn test_todolist_read_done() {
        let todo_list = get_todo_list();
        let expected_done = vec![Todo::written("this one is 2 and done".to_string(), 2, true),Todo::written("this one is 0 and done".to_string(), 0, true)];
        assert_eq!(expected_done, todo_list.todos.iter().filter(|todo| todo.done()).cloned().collect::<Vec<Todo>>());
    }

    #[test]
    fn test_len() {
        let todo_list = get_todo_list();
        assert_eq!(todo_list.len(None), 4);
    }

    #[test]
    fn test_write() {
        let mut todo_list = get_todo_list();
        let path = PathBuf::from("todo-list-test-write/tmplist");
        let _ = todo_list.write(&path, true);


        let contents = fs::read_to_string(&path).expect("Reading file failed :(");
        let expected = "[1] this todo has prio 1
[2] this one has prio 2
[-2] this one is 2 and done
[-0] this one is 0 and done
";

        remove_dir_all(&path.parent().unwrap()).expect("Remove test failed");
        let _ = remove_file(path);
        assert_eq!(contents, expected)
    }

    #[test]
    fn test_push() {
        let mut todo_list = get_todo_list();
        let path = PathBuf::from("todo-list-test-push/tmplist");
        todo_list.push(Todo::default("Show me your warface".to_string(), 0));
        todo_list.reorder_last();
        let _ = todo_list.write(&path, true);

        let contents = fs::read_to_string(&path).expect("Reading file failed :(");
        let expected = "[1] this todo has prio 1
[2] this one has prio 2
[0] Show me your warface
[-2] this one is 2 and done
[-0] this one is 0 and done
";

        remove_dir_all(&path.parent().unwrap()).expect("Remove test failed");
        let _ = remove_file(path);
        assert_eq!(contents, expected);
    }

    #[test]
    fn test_initially_sorted() {
        let todo_list = get_todo_list();
        let mut sorted_list = todo_list.clone();
        sorted_list.sort();

        assert_eq!(todo_list, sorted_list)
    }

    #[test]
    fn test_write_dependencies() -> io::Result<()>{
        let mut todo_list = get_todo_list();
        let _ = todo_list.todos[0].add_todo_dependency();
        let path = PathBuf::from("test-write-dependency/tmplist");
        todo_list.todos[0].dependency.push(Todo::try_from("[0] Some dependency").unwrap());
        let dependency_path = todo_list.write(&path, true)?;
        todo_list.write_dependencies(&dependency_path)?;

        let todo_dependency_path = PathBuf::from(format!("test-write-dependency/notes/{}.todo", todo_list.todos[0].hash()));
        let contents = fs::read_to_string(&todo_dependency_path).expect("Reading file failed :(");
        let expected = "[0] Some dependency\n";
        assert_eq!(contents, expected);

        todo_list.todos[0].remove_dependency();
        todo_list.write(&path, true)?;
        remove_dir_all(&path.parent().unwrap())?;
        Ok(())
    }
}
