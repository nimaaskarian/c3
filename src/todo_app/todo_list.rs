use std::fs::{read, File};
use std::io;
use std::io::{stdout, BufRead, BufWriter, Write};
use std::path::Path;

use super::{App, RestrictionFunction, Todo};
use crate::DisplayArgs;

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct TodoList {
    pub todos: Vec<Todo>,
    pub(super) changed: bool,
}

type Output = Todo;

fn with_index<T, F>(mut f: F) -> impl FnMut(&T) -> bool
where
    F: FnMut(usize, &T) -> bool,
{
    let mut i = 0;
    move |item| (f(i, item), i += 1).0
}

impl TodoList {
    pub fn new() -> Self {
        TodoList {
            ..Default::default()
        }
    }

    pub fn index(&self, index: usize, restriction: &RestrictionFunction) -> &Output {
        self.todos
            .iter()
            .filter(|todo| restriction(todo))
            .nth(index)
            .unwrap()
    }

    pub fn index_mut(&mut self, index: usize, restriction: &RestrictionFunction) -> &mut Output {
        self.changed = true;
        self.todos
            .iter_mut()
            .filter(|todo| restriction(todo))
            .nth(index)
            .unwrap()
    }

    pub fn todos(&self, restriction: &RestrictionFunction) -> Vec<&Todo> {
        self.todos.iter().filter(|todo| restriction(todo)).collect()
    }

    pub fn todos_mut(&mut self, restriction: &RestrictionFunction) -> Vec<&mut Todo> {
        self.todos
            .iter_mut()
            .filter(|todo| restriction(todo))
            .collect()
    }

    #[inline]
    pub(super) fn delete_removed_dependent_files(&mut self, filename: &Path) -> io::Result<()> {
        for todo in &mut self.todos {
            if let Some(dependency) = todo.dependency.as_mut() {
                dependency
                    .todo_list
                    .delete_removed_dependent_files(filename)?;
            }
            todo.delete_removed_dependent_files(filename)?;
        }
        Ok(())
    }

    #[inline]
    pub fn prepend(&mut self, todo: Todo) {
        self.changed = true;
        self.todos.insert(0, todo);
    }

    #[inline]
    pub fn write_to_stdout(&self) -> io::Result<()> {
        let mut stdout_writer = BufWriter::new(stdout());
        self.write_to_buf(&mut stdout_writer)?;
        Ok(())
    }

    pub fn traverse_tree(
        &self,
        callback: fn(&mut App, &TodoList, &[usize]),
        prior_indices: Vec<usize>,
        app: &mut App,
    ) {
        callback(app, self, &prior_indices);
        for (i, todo) in self.todos.iter().enumerate() {
            if let Some(todo_list) = todo.dependency.as_ref().and_then(|dep| dep.todo_list()) {
                let mut prior_indices = prior_indices.clone();
                prior_indices.push(i);
                todo_list.traverse_tree(callback, prior_indices, app);
            }
        }
    }

    pub(super) fn remove_dependency_files(&mut self, filename: &Path) -> io::Result<()> {
        for todo in &mut self.todos {
            todo.delete_dependency_file(filename)?;
        }
        Ok(())
    }

    pub fn read(filename: &Path) -> Self {
        if !filename.is_file() {
            return Self::new();
        }
        let file_data = read(filename).unwrap();
        let mut todolist = Self {
            todos: file_data
                .lines()
                .map_while(Result::ok)
                .flat_map(|line| line.parse())
                .collect(),
            ..Default::default()
        };
        todolist.sort();
        todolist.changed = false;
        todolist
    }

    pub fn read_dependencies(&mut self, folder_name: &Path) -> io::Result<()> {
        for todo in &mut self.todos {
            if let Some(dependency) = todo.dependency.as_mut() {
                dependency.read(folder_name)?;
            }
        }
        Ok(())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        TodoList {
            todos: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    #[inline]
    fn write_to_buf<W: Write>(&self, writer: &mut BufWriter<W>) -> io::Result<()> {
        for todo in &self.todos {
            writeln!(writer, "{}", todo.as_string())?;
        }
        writer.flush()?;
        Ok(())
    }

    #[inline]
    pub(super) fn write_dependencies(&mut self, filename: &Path) -> io::Result<()> {
        for todo in &mut self.todos {
            if let Some(dependency) = todo.dependency.as_mut() {
                dependency.todo_list.write_dependencies(filename)?;
                dependency.write(filename)?;
            }
        }
        Ok(())
    }

    #[inline]
    pub(super) fn force_write_dependencies(&mut self, filename: &Path) -> io::Result<()> {
        for todo in &mut self.todos {
            if let Some(dependency) = todo.dependency.as_mut() {
                dependency.todo_list.force_write_dependencies(filename)?;
                dependency.force_write(filename)?;
            }
        }
        Ok(())
    }

    #[inline]
    pub fn force_write(&mut self, filename: &Path) -> io::Result<()> {
        let file = File::create(filename)?;
        let mut writer = BufWriter::new(file);
        self.write_to_buf(&mut writer)?;
        self.changed = false;
        Ok(())
    }

    #[inline]
    pub fn write(&mut self, filename: &Path) -> io::Result<()> {
        if self.changed {
            self.force_write(filename)?;
        }
        Ok(())
    }

    #[inline(always)]
    pub(super) fn filter_indices(&mut self, sorted_indices: Vec<usize>) {
        self.todos
            .retain(with_index(|i, _| sorted_indices.binary_search(&i).is_err()))
    }

    pub fn iter(&self) -> std::slice::Iter<Todo> {
        self.todos.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<Todo> {
        self.changed = true;
        self.todos.iter_mut()
    }

    pub fn messages(&self, restriction: &RestrictionFunction) -> Vec<&str> {
        self.todos
            .iter()
            .filter(|todo| restriction(todo))
            .map(|todo| todo.message.as_str())
            .collect()
    }

    pub fn filter<'a>(
        &'a self,
        restriction: &'a RestrictionFunction,
    ) -> std::iter::Filter<std::slice::Iter<Todo>, impl FnMut(&&'a Todo) -> bool> {
        self.todos.iter().filter(|todo| restriction(todo))
    }

    pub fn display(&self, args: &DisplayArgs, restriction: &RestrictionFunction) -> Vec<String> {
        self.todos
            .iter()
            .filter(|todo| restriction(todo))
            .map(|todo| todo.display(args))
            .collect()
    }

    pub fn display_slice(
        &self,
        args: &DisplayArgs,
        restriction: &RestrictionFunction,
        min: usize,
        max: usize,
    ) -> Vec<String> {
        self.todos
            .iter()
            .filter(|todo| restriction(todo))
            .skip(min)
            .take(max)
            .map(|todo| todo.display(args))
            .collect()
    }

    pub fn len(&self, restriction: &RestrictionFunction) -> usize {
        self.todos.iter().filter(|todo| restriction(todo)).count()
    }

    pub fn is_empty(&self, restriction: &RestrictionFunction) -> bool {
        self.len(restriction) == 0
    }

    pub fn true_position_in_list(&self, index: usize, restriction: &RestrictionFunction) -> usize {
        let binding = self.todos(restriction);
        self.todos
            .iter()
            .position(|x| x == binding[index])
            .unwrap_or_default()
    }

    pub fn remove(&mut self, index: usize, restriction: &RestrictionFunction) -> Todo {
        self.changed = true;
        let index_in_vec = self.true_position_in_list(index, restriction);
        self.todos.remove(index_in_vec)
    }

    pub fn push(&mut self, item: Todo) {
        self.changed = true;
        self.todos.push(item);
    }

    #[inline(always)]
    fn reorder_low_high(&self, index: usize) -> (usize, usize) {
        if index + 1 < self.todos.len() && self.todos[index] > self.todos[index + 1] {
            (index + 1, self.todos.len() - 1)
        } else {
            (0, index)
        }
    }

    #[inline(always)]
    pub fn reorder_last(&mut self) -> usize {
        self.changed = true;
        self.reorder(self.todos.len() - 1)
    }

    pub fn reorder(&mut self, index: usize) -> usize {
        self.changed = true;
        if self.todos[index] < self.todos[0] {
            return self.move_index(index, 0, 1);
        }

        let (low, high) = self.reorder_low_high(index);
        for i in low..high {
            if self.todos[index] < self.todos[i + 1] && self.todos[index] >= self.todos[i] {
                return self.move_index(index, i, 0);
            }
        }
        self.move_index(index, high, 0)
    }

    #[inline(always)]
    fn move_index(&mut self, from: usize, to: usize, shift: usize) -> usize {
        let mut i = from;
        if from < to {
            for j in from..to {
                self.todos.swap(j, j + 1);
                i = j + 1;
            }
        } else {
            for j in (to + 1 - shift..from).rev() {
                self.todos.swap(j, j + 1);
                i = j;
            }
        }
        i
    }

    pub fn append_list(&mut self, mut todo_list: TodoList) {
        self.changed = true;
        self.todos.append(&mut todo_list.todos);
        self.sort();
    }

    #[inline(always)]
    pub fn sort(&mut self) {
        self.changed = true;
        self.todos.sort()
    }
}

#[cfg(test)]
mod tests {
    use crate::fileio;
    use std::fs::{self, create_dir_all, remove_dir_all, remove_file};
    use std::path::PathBuf;
    use std::str::FromStr;

    use super::*;

    fn get_todo_list() -> TodoList {
        let path = PathBuf::from("tests/TODO_LIST");
        let mut todolist = TodoList::read(&path);
        todolist.read_dependencies(&path).expect("reading todo dependencies failed");
        todolist
    }

    #[test]
    fn test_todolist_read_undone() {
        let todo_list = get_todo_list();
        let expected_undone = vec![
            Todo::new("this todo has prio 1".to_string(), 1),
            Todo::new("this one has prio 2".to_string(), 2),
        ];

        assert_eq!(
            expected_undone,
            todo_list
                .todos
                .iter()
                .filter(|todo| !todo.done())
                .cloned()
                .collect::<Vec<Todo>>()
        );
    }

    #[test]
    fn test_todolist_read_done() {
        let todo_list = get_todo_list();
        let mut expected_done = vec![
            Todo::new("this one is 2 and done".to_string(), 2),
            Todo::new("this one is 0 and done".to_string(), 0),
        ];
        for i in 0..expected_done.len() {
            expected_done[i].toggle_done();
        }
        assert_eq!(
            expected_done,
            todo_list
                .todos
                .iter()
                .filter(|todo| todo.done())
                .cloned()
                .collect::<Vec<Todo>>()
        );
    }

    #[test]
    fn test_len() {
        let todo_list = get_todo_list();
        assert_eq!(todo_list.len(&App::no_restriction()), 4);
    }

    #[test]
    fn test_write() {
        let mut todo_list = get_todo_list();
        let path = PathBuf::from("todo-list-test-write/tmplist");
        let dependency_path = fileio::append_notes_to_path_parent(&path);
        let _ = create_dir_all(&dependency_path);
        todo_list.changed = true;

        let _ = todo_list.write(&path);

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
        let _ = create_dir_all(&path.parent().unwrap());
        todo_list.push(Todo::new("Show me your warface".to_string(), 0));
        todo_list.reorder_last();
        let _ = todo_list.write(&path);

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
        sorted_list.changed = false;

        assert_eq!(todo_list, sorted_list)
    }

    #[test]
    fn test_write_dependencies() -> io::Result<()> {
        let mut todo_list = get_todo_list();
        let _ = todo_list.todos[0].add_todo_dependency();

        let path = PathBuf::from("test-write-dependency/tmplist");
        let dependency_path = fileio::append_notes_to_path_parent(&path);
        let _ = create_dir_all(&dependency_path);

        todo_list.todos[0]
            .dependency
            .as_mut()
            .unwrap()
            .push(Todo::from_str("[0] Some dependency").unwrap());
        todo_list.write(&path)?;
        let dependency_path = fileio::append_notes_to_path_parent(&path);
        todo_list.write_dependencies(&dependency_path)?;

        let todo_dependency_path = PathBuf::from(format!(
            "test-write-dependency/notes/{}.todo",
            todo_list.todos[0].hash()
        ));
        let contents = fs::read_to_string(&todo_dependency_path).expect("Reading file failed :(");
        let expected = "[0] Some dependency\n";
        assert_eq!(contents, expected);

        todo_list.todos[0].remove_dependency();
        todo_list.write(&path)?;
        remove_dir_all(&path.parent().unwrap())?;
        Ok(())
    }
}
