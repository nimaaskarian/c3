use std::collections::VecDeque;
use std::cmp;
use std::fs::create_dir_all;
use std::path::Path;
use std::str::{FromStr, Lines};
use std::{io, path::PathBuf};
mod clipboard;
use clipboard::Clipboard;
mod todo;
mod todo_list;
use crate::{fileio, AppArgs, SortMethod};
use std::rc::Rc;
pub use todo::Todo;

pub use self::todo_list::TodoList;

#[derive(Clone)]
struct SearchPosition {
    tree_path: Vec<usize>,
    matching_indices: Vec<usize>,
}

pub(crate) fn ord_by_abandonment_coefficient(a: &Todo, b: &Todo) -> cmp::Ordering {
    let order = b.abandonment_coefficient().total_cmp(&a.abandonment_coefficient());
    if order.is_eq() {
        return a.cmp(b)
    }
    order
}


pub type Restriction = Rc<dyn Fn(&Todo) -> bool>;
pub struct App {
    notes_dir: PathBuf,
    clipboard: Clipboard,
    pub(super) todo_list: TodoList,
    pub(super) index: usize,
    changed: bool,
    tree_path: Vec<usize>,
    pub(super) args: AppArgs,
    removed_todos: Vec<Todo>,
    tree_search_positions: Vec<SearchPosition>,
    x_index: usize,
    y_index: usize,
    restriction: Restriction,
}

#[derive(Debug)]
struct IndexedLine {
    message: String,
    priority: u8,
    index: Option<usize>,
}

#[derive(Debug)]
struct LineMalformed;

impl FromStr for IndexedLine {
    type Err = LineMalformed;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (mut index, message) = first_word_parse(input);
        let (mut priority, message) = first_word_parse(message.as_str());
        if priority.is_none() {
            priority = index.map(|val| val as u8);
            index = None;
        }

        Ok(Self {
            index,
            message,
            priority: priority.unwrap_or_default(),
        })
    }
}

fn first_word_parse<T: FromStr>(input: &str) -> (Option<T>, String) {
    let input = input.trim_start();
    let position = input
        .chars()
        .position(|x| x.is_whitespace())
        .unwrap_or(input.len());
    match input[..position].parse::<T>() {
        Ok(num) => {
            let rest = input[position..].trim_start().to_string();
            (Some(num), rest)
        }
        Err(_) => (None, input.to_string()),
    }
}

impl App {
    #[inline]
    pub(crate) fn new(args: AppArgs) -> Self {
        let notes_dir = fileio::append_notes_to_path_parent(&args.todo_path);
        let todo_list = Self::read_a_todo_list(&args.todo_path, &notes_dir, &args);
        let mut app = App {
            notes_dir,
            x_index: 0,
            y_index: 0,
            tree_search_positions: vec![],
            removed_todos: vec![],
            todo_list,
            clipboard: Clipboard::new(),
            index: 0,
            tree_path: vec![],
            changed: false,
            args,
            restriction: Self::no_restriction(),
        };
        app.update_show_done_restriction();
        app
    }

    pub fn toggle_schedule(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.toggle_schedule();
        }
    }

    #[inline(always)]
    fn read_a_todo_list(path: &Path, notes_dir: &Path, args: &AppArgs) -> TodoList {
        let mut todo_list = TodoList::read(path);
        if args.sort_method == SortMethod::AbandonedFirst {
            todo_list.set_todo_cmp(ord_by_abandonment_coefficient);
            todo_list.sort();
            todo_list.changed = false;
        }
        if !args.no_tree {
            todo_list.read_dependencies(notes_dir);
        }
        todo_list
    }

    #[inline]
    pub fn append_list_from_path(&mut self, path: &Path) {
        let notes_dir = fileio::append_notes_to_path_parent(path);
        let todo_list = Self::read_a_todo_list(path, &notes_dir, &self.args);
        self.append_list(todo_list)
    }

    #[inline]
    pub fn restriction(&self) -> &Restriction {
        &self.restriction
    }

    #[inline]
    pub fn open_path(&mut self, path: PathBuf) {
        self.notes_dir = fileio::append_notes_to_path_parent(&path);
        self.todo_list = Self::read_a_todo_list(&path, &self.notes_dir, &self.args);
        self.tree_path = vec![];
        self.args.todo_path = path;
    }

    #[inline]
    pub fn output_list_to_path(&self, path: &Path) -> io::Result<()> {
        let list = self.current_list();
        let dependency_path = fileio::append_notes_to_path_parent(path);
        create_dir_all(&dependency_path)?;
        list.force_write(path)?;

        list.force_write_dependencies(&dependency_path)?;
        Ok(())
    }

    #[inline]
    pub fn append_list(&mut self, todo_list: TodoList) {
        self.current_list_mut().append_list(todo_list)
    }

    pub fn set_query_restriction(&mut self, query: String, last_restriction: Option<Restriction>) {
        let last_restriction = last_restriction.unwrap_or(self.restriction.clone());
        self.set_restriction(Rc::new(move |todo| {
            todo.matches(query.as_str()) && last_restriction(todo)
        }))
    }

    pub fn tree_search(&mut self, query: String) {
        self.tree_search_positions = vec![];
        self.y_index = 0;
        self.x_index = 0;
        if query.is_empty() {
            return;
        }
        let current_not_matches = self.todo().map_or(true, |todo| !todo.matches(&query));
        self.search_tree(query);

        if current_not_matches {
            self.search_next();
        }
    }

    pub fn search_tree(&mut self, query: String) {
        let mut lists: VecDeque<(Vec<usize>, &TodoList)> =
            VecDeque::from([(vec![], &self.todo_list)]);
        while let Some((indices, current_list)) = lists.pop_back() {
            let mut matching_indices: Vec<usize> = vec![];
            for (i, todo) in current_list.filter(&self.restriction).enumerate() {
                let mut todo_indices = indices.clone();
                todo_indices.push(i);
                if todo.matches(&query) {
                    matching_indices.push(i)
                }
                if let Some(list) = todo.dependency.as_ref().and_then(|dep| dep.todo_list()) {
                    lists.push_back((todo_indices, list))
                }
            }
            if !matching_indices.is_empty() {
                self.tree_search_positions.push(SearchPosition {
                    tree_path: indices.to_vec(),
                    matching_indices,
                })
            }
        }
    }

    pub fn batch_editor_messages(&mut self) {
        let content = String::from("# INDEX PRIORITY MESSAGE\n")
            + self
                .current_list()
                .filter(&self.restriction)
                .enumerate()
                .map(|(i, x)| format!("{i: <7} {: <8} {}", x.priority(), x.message))
                .collect::<Vec<String>>()
                .join("\n")
                .as_str();
        let new_messages =
            fileio::open_temp_editor(Some(&content), fileio::temp_path("messages")).unwrap();
        let new_messages = new_messages.lines();
        self.batch_edit_current_list(new_messages)
    }

    #[inline(always)]
    fn batch_edit_current_list(&mut self, messages: Lines<'_>) {
        let restriction = self.restriction.clone();
        let mut lines: Vec<IndexedLine> = messages
            .filter(|message| !message.starts_with('#'))
            .flat_map(|message| message.parse())
            .collect();

        lines.sort_by_key(|a| a.index);
        let todolist = self.current_list_mut();
        let size = todolist.len(&restriction);
        let indices: Vec<usize> = lines.iter().filter_map(|x| x.index).collect();
        let delete_indices: Vec<usize> = (0..size)
            .filter(|i| indices.binary_search(i).is_err())
            .collect();
        let mut changed = !delete_indices.is_empty();

        for line in lines {
            if let Some(index) = line.index {
                let index = todolist.true_position_in_list(index, &restriction);
                let todo = &mut todolist.todos[index];
                if line.priority != todo.priority() || line.message != todo.message {
                    changed = true;
                    todo.set_message(line.message);
                    todo.set_priority(line.priority);
                }
            } else {
                todolist.push(Todo::new(line.message, line.priority));
            }
        }
        todolist.retrain_indices(delete_indices);
        todolist.changed = todolist.changed || changed;
        if todolist.changed {
            todolist.sort();
        }
        self.changed = todolist.changed;
    }

    #[inline]
    pub fn is_tree(&self) -> bool {
        !self.args.no_tree
    }

    #[inline]
    pub fn is_current_changed(&self) -> bool {
        self.current_list().changed
    }

    #[inline]
    pub fn is_changed(&self) -> bool {
        self.changed
    }

    #[inline]
    pub fn increase_day_done(&mut self) {
        if let Some(Some(schedule)) = self.todo_mut().map(|todo| todo.schedule.as_mut()) {
            schedule.add_days_to_date(-1);
            self.reorder_current();
        }
    }

    #[inline]
    pub fn decrease_day_done(&mut self) {
        if let Some(Some(schedule)) = self.todo_mut().map(|todo| todo.schedule.as_mut()) {
            schedule.add_days_to_date(1);
            self.reorder_current();
        }
    }

    #[inline]
    pub fn prepend(&mut self, message: String) {
        self.current_list_mut().prepend(Todo::new(message, 1));
        self.index = 0;
    }

    #[inline]
    pub fn append(&mut self, message: String) {
        let todo_list = self.current_list_mut();
        todo_list.push(Todo::new(message, 0));
        self.index = todo_list.reorder_last();
    }

    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn show_done(&self) -> bool {
        self.args.display_args.show_done
    }

    #[inline]
    pub fn toggle_show_done(&mut self) {
        self.args.display_args.show_done = !self.show_done();
        self.update_show_done_restriction();
    }

    pub fn update_show_done_restriction(&mut self) {
        if self.show_done() {
            self.unset_restriction()
        } else {
            self.set_restriction(Rc::new(|todo| !todo.done()))
        }
    }

    #[inline]
    pub fn search_next(&mut self) {
        if !self.tree_search_positions.is_empty() {
            let x_size = self.tree_search_positions.len();
            let y_size = self.tree_search_positions[self.x_index]
                .matching_indices
                .len();
            if self.x_index + 1 < x_size {
                self.x_index += 1
            } else if self.y_index + 1 < y_size {
                self.y_index += 1
            } else {
                self.y_index = 0;
                self.x_index = 0;
            }
            self.set_tree_search_position();
        }
    }

    #[inline]
    fn set_tree_search_position(&mut self) {
        let item = &self.tree_search_positions[self.x_index];
        self.tree_path.clone_from(&item.tree_path);
        self.index = item.matching_indices[self.y_index];
    }

    fn max_tree_length(&self) -> usize {
        let mut current_list = &self.todo_list;
        let mut max_i = 0;
        for &index in self.tree_path.iter() {
            if let Some(dependency) = current_list
                .todos
                .get(index)
                .and_then(|todo| todo.dependency.as_ref())
            {
                current_list = &dependency.todo_list;
                max_i += 1;
            } else {
                break;
            }
        }
        max_i
    }

    #[inline]
    pub fn toggle_current_done(&mut self) {
        let index = self.index;
        self.todo_mut().unwrap().toggle_done();
        if self.show_done() {
            self.index = self.current_list_mut().reorder(index);
        } else {
            self.current_list_mut().sort();
            self.fix_index();
        }
        while self.is_undone_empty() && self.traverse_up() {
            self.toggle_current_done()
        }
    }

    #[inline]
    pub fn read(&mut self) {
        self.changed = false;
        self.todo_list = Self::read_a_todo_list(&self.args.todo_path, &self.notes_dir, &self.args);
        let len = self.max_tree_length();
        self.tree_path.truncate(len);
    }

    #[inline]
    pub fn fix_index(&mut self) {
        let size = self.current_list().len(&self.restriction);
        self.index = match size {
            0 => 0,
            _ => self.index.min(size - 1),
        }
    }

    #[inline]
    pub fn parent(&mut self) -> Option<&Todo> {
        let mut list = &self.todo_list;
        let mut parent = None;
        for &index in &self.tree_path {
            parent = Some(&list.todos[index]);
            if let Some(todo_list) = list.todos[index]
                .dependency
                .as_ref()
                .and_then(|dep| dep.todo_list())
            {
                list = todo_list
            } else {
                break;
            }
        }
        parent
    }

    #[inline]
    pub fn increment(&mut self) {
        let size = self.len();
        if size == 0 || self.index == size - 1 {
            self.index = 0;
        } else {
            self.index += 1
        }
    }

    #[inline]
    pub fn decrement(&mut self) {
        if self.index != 0 {
            self.index -= 1;
        } else {
            self.go_bottom()
        }
    }

    #[inline]
    pub fn traverse_down(&mut self) {
        if self.is_tree() {
            match self.todo() {
                Some(todo) if todo.dependency.as_ref().map_or(false, |dep| dep.is_list()) => {
                    let index = self.index;
                    let restriction = self.restriction.clone();
                    let true_index = self
                        .current_list()
                        .true_position_in_list(index, &restriction);
                    self.tree_path.push(true_index);
                    self.index = 0;
                    self.update_show_done_restriction();
                }
                _ => {}
            }
        }
    }

    #[inline]
    pub fn go_root(&mut self) {
        self.tree_path = vec![];
        self.fix_index();
    }

    #[inline]
    pub fn traverse_up(&mut self) -> bool {
        self.update_show_done_restriction();
        if let Some(index) = self.tree_path.pop() {
            self.index = index;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn go_bottom(&mut self) {
        self.index = self.bottom();
    }

    #[inline]
    pub fn bottom(&self) -> usize {
        match self.len() {
            0 => 0,
            length => length - 1,
        }
    }

    #[inline]
    pub fn is_todos_empty(&self) -> bool {
        self.current_list().is_empty(&self.restriction)
    }

    #[inline]
    pub fn todo_mut(&mut self) -> Option<&mut Todo> {
        if self.is_todos_empty() {
            return None;
        }
        let index = self.index.min(self.len() - 1);
        let size = self.len();
        let res_cloned = self.restriction.clone();

        if size <= index {
            return Some(self.current_list_mut().index_mut(size - 1, &res_cloned));
        }

        Some(self.current_list_mut().index_mut(index, &res_cloned))
    }

    #[inline]
    pub fn cut_todo(&mut self) {
        if !self.is_todos_empty() {
            let restriction = self.restriction.clone();
            let index = self.index;
            let todo = self.current_list_mut().remove(index, &restriction);
            let todo_string: String = (&todo).into();
            self.clipboard.set_text(todo_string);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.current_list().len(&self.restriction)
    }

    #[inline]
    pub fn current_list_mut(&mut self) -> &mut TodoList {
        self.changed = true;
        let is_root = self.is_root();
        if is_root {
            return &mut self.todo_list;
        }
        let mut list = &mut self.todo_list;

        for &index in &self.tree_path {
            if list.todos[index].dependency.is_some() {
                list = &mut list.todos[index].dependency.as_mut().unwrap().todo_list
            }
        }
        list
    }

    #[inline]
    pub fn current_list(&self) -> &TodoList {
        let mut list = &self.todo_list;
        if self.is_root() {
            return list;
        }
        for &index in &self.tree_path {
            if let Some(todo_list) = &list.todos[index]
                .dependency
                .as_ref()
                .and_then(|dep| dep.todo_list())
            {
                list = todo_list
            } else {
                break;
            }
        }
        list
    }

    #[inline]
    pub fn handle_removed_todo_dependency_files(&mut self, dependency_path: &Path) {
        for todo in &mut self.removed_todos {
            let _ = todo.delete_dependency_file(dependency_path);
        }
        self.removed_todos = vec![];
    }

    #[inline]
    pub fn write(&mut self) -> io::Result<()> {
        let note_dir = fileio::append_notes_to_path_parent(&self.args.todo_path);

        create_dir_all(&note_dir)?;
        let todo_path = self.args.todo_path.clone();
        self.handle_removed_todo_dependency_files(&note_dir);
        self.todo_list.write(&todo_path)?;
        self.todo_list.delete_removed_dependent_files(&note_dir)?;
        if self.is_tree() {
            self.todo_list.write_dependencies(&note_dir)?;
        }
        self.changed = false;
        Ok(())
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.tree_path.is_empty()
    }

    #[inline]
    pub fn only_undone_empty(&self) -> bool {
        self.is_undone_empty() && !self.is_done_empty()
    }

    #[inline]
    pub fn toggle_current_daily(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.toggle_daily();
            self.reorder_current();
        }
    }

    #[inline]
    pub fn toggle_current_weekly(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.toggle_weekly();
            self.reorder_current();
        }
    }

    #[inline]
    pub fn is_undone_empty(&self) -> bool {
        let restriction: Restriction = Rc::new(move |todo| !todo.done());
        self.current_list().is_empty(&restriction)
    }

    #[inline]
    pub fn is_done_empty(&self) -> bool {
        let restriction: Restriction = Rc::new(move |todo| todo.done());
        self.current_list().is_empty(&restriction)
    }

    #[inline(always)]
    pub fn no_restriction() -> Restriction {
        Rc::new(|_| true)
    }

    #[inline(always)]
    pub fn unset_restriction(&mut self) {
        self.restriction = Self::no_restriction();
    }

    #[inline(always)]
    pub fn set_restriction(&mut self, restriction: Restriction) {
        self.restriction = restriction;
        self.fix_index();
    }

    #[inline]
    pub fn set_priority_restriction(
        &mut self,
        priority: u8,
        last_restriction: Option<Restriction>,
    ) {
        let last_restriction = last_restriction.unwrap_or(self.restriction.clone());
        self.set_restriction(Rc::new(move |todo| {
            todo.priority() == priority && last_restriction(todo)
        }))
    }

    #[inline]
    pub fn set_priority_limit_no_done(&mut self, priority: u8) {
        self.args.display_args.show_done = false;
        self.set_restriction(Rc::new(move |todo| {
            todo.priority() == priority && !todo.done()
        }))
    }

    #[inline]
    pub fn set_current_priority(&mut self, priority: u8) {
        if let Some(todo) = self.todo_mut() {
            todo.set_priority(priority);
            self.reorder_current();
        }
    }

    #[inline]
    pub fn get_cloned_current_message(&mut self) -> Option<String> {
        self.todo().map(|todo| todo.message.clone())
    }

    #[inline]
    pub fn todo(&self) -> Option<&Todo> {
        if self.is_todos_empty() {
            return None;
        }

        let current_list = self.current_list();
        let index = self.index.min(self.len() - 1);
        let size = self.len();

        if size <= index {
            return Some(current_list.index(size - 1, &self.restriction));
        }

        Some(self.current_list().index(index, &self.restriction))
    }

    #[inline]
    pub fn reorder_current(&mut self) {
        let index = self.index;
        self.index = self.current_list_mut().reorder(index);
    }

    #[inline]
    pub fn delete_todo(&mut self) {
        let restriction = self.restriction.clone();
        if !self.is_todos_empty() {
            let index = self.index;
            let todo = self.current_list_mut().remove(index, &restriction);
            self.removed_todos.push(todo);
        }
    }

    #[inline]
    pub fn display_current(&self) -> Vec<String> {
        self.display_list(self.current_list())
    }

    #[inline]
    pub fn display_current_slice(&self, min: usize, max: usize) -> Vec<String> {
        self.current_list()
            .display_slice(&self.args.display_args, &self.restriction, min, max)
    }

    #[inline]
    pub fn display_list(&self, todo_list: &TodoList) -> Vec<String> {
        todo_list.display(&self.args.display_args, &self.restriction)
    }

    #[inline]
    pub fn remove_current_dependent(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.remove_dependency();
        }
    }

    #[inline]
    pub fn add_dependency(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.add_todo_dependency();
        }
    }

    #[inline]
    pub fn edit_or_add_note(&mut self) {
        if self.is_tree() {
            let list_changed = self.current_list().changed;
            let changed = self.changed;
            if let Some(todo) = self.todo_mut() {
                if !todo.edit_note().unwrap_or_default() {
                    self.current_list_mut().changed = list_changed;
                    self.changed = changed;
                }
            }
        }
    }

    #[inline]
    pub fn decrease_current_priority(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.decrease_priority();
            self.reorder_current();
        }
    }

    #[inline]
    pub fn increase_current_priority(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.increase_priority();
            self.reorder_current();
        }
    }

    #[inline]
    pub fn yank_todo(&mut self) {
        if let Some(todo) = self.todo() {
            let todo_string: String = todo.into();
            self.clipboard.set_text(todo_string);
        }
    }

    #[inline]
    pub fn paste_todo(&mut self) {
        if let Ok(mut todo) = self.clipboard.get_text().parse::<Todo>() {
            if let Some(dependency) = todo.dependency.as_mut() {
                if dependency.is_written() {
                    let _ = dependency.read(&self.notes_dir, self.todo_list.todo_cmp);
                }
            }
            let list = &mut self.current_list_mut();
            list.push(todo);
            self.index = list.reorder_last();
        }
    }

    #[inline]
    pub fn add_dependency_traverse_down(&mut self) {
        if self.is_tree() {
            // The reason we are using a self.todo() here, is that if we don't want to
            // change anything, we won't borrow mutable and set the self.changed=true
            if let Some(todo) = self.todo() {
                if todo.dependency.is_none() {
                    self.todo_mut().unwrap().add_todo_dependency();
                }
            }
            self.traverse_down()
        }
    }

    #[inline]
    pub fn write_to_stdout(&self) -> io::Result<()> {
        self.todo_list.write_to_stdout()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, remove_dir_all},
        path::Path,
    };

    use clap::Parser;

    use super::*;

    fn dir(dir_name: &str) -> io::Result<PathBuf> {
        let path = PathBuf::from(dir_name);
        fs::create_dir_all(path.join("notes"))?;
        Ok(path)
    }

    fn write_test_todos(dir: &Path) -> io::Result<App> {
        let mut args = AppArgs::parse();
        fs::create_dir_all(dir.join("notes"))?;
        args.todo_path = dir.join("todo");
        let mut app = App::new(args);
        app.append(String::from("Hello"));
        app.append(String::from("Goodbye"));
        app.append(String::from("Hello there"));
        let dependencies = vec![
            "Is there anybody outthere?",
            "Just nod if you can here me",
            "Is there anyone home",
        ];
        for dependency in dependencies {
            app.add_dependency_traverse_down();
            app.append(String::from(dependency));
        }
        app.todo_mut()
            .unwrap()
            .set_note("Heaven from hell".to_string())
            .unwrap();
        for _ in 0..3 {
            app.traverse_up();
        }
        app.write()?;
        Ok(app)
    }

    #[test]
    fn test_is_changed() -> io::Result<()> {
        let dir = dir("test-is-changed")?;
        let mut app = write_test_todos(&dir)?;
        assert_eq!(app.is_changed(), false);
        app.todo_mut();
        assert_eq!(app.is_changed(), true);
        app.write()?;
        assert_eq!(app.is_changed(), false);
        app.current_list_mut();
        assert_eq!(app.is_changed(), true);
        app.read();
        assert_eq!(app.is_changed(), false);
        remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn test_set_restrictions_done() -> io::Result<()> {
        let dir = dir("test-set-restrictions-done")?;
        let mut app = write_test_todos(&dir)?;
        app.toggle_current_done();
        assert_eq!(app.len(), 2);
        app.toggle_show_done();
        assert_eq!(app.len(), 3);
        app.toggle_show_done();
        assert_eq!(app.len(), 2);
        remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn test_set_restrictions_query() -> io::Result<()> {
        let dir = dir("test-set-restrictions-query")?;
        let mut app = write_test_todos(&dir)?;
        assert_eq!(app.len(), 3);
        app.set_query_restriction(String::from("hello"), None);
        assert_eq!(app.len(), 2);
        assert_eq!(app.index, 1);
        app.traverse_down();
        app.unset_restriction();
        app.traverse_up();
        app.set_query_restriction(String::from("hello"), None);
        assert_eq!(app.len(), 2);
        assert_eq!(app.index, 1);
        app.add_dependency_traverse_down();
        assert_eq!(app.len(), 1);
        remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn test_set_restrictions_priority() -> io::Result<()> {
        let dir = dir("test-set-restrictions-priority")?;
        let mut app = write_test_todos(&dir)?;
        app.set_current_priority(2);
        assert_eq!(app.len(), 3);
        app.set_priority_restriction(2, None);
        assert_eq!(app.len(), 1);
        app.set_priority_restriction(0, None);
        assert_eq!(app.len(), 0);
        app.update_show_done_restriction();
        app.set_priority_restriction(0, None);
        assert_eq!(app.len(), 2);
        remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn test_tree_search() -> io::Result<()> {
        let dir = dir("test-tree-search")?;
        let mut app = write_test_todos(&dir)?;
        remove_dir_all(dir)?;
        let query = String::from("nod");
        app.tree_search(query);
        let position = &app.tree_search_positions[0];
        assert_eq!(position.tree_path, vec![2, 0]);
        assert_eq!(position.matching_indices, vec![0]);
        Ok(())
    }

    #[test]
    fn test_write() -> io::Result<()> {
        let dir = dir("test-write")?;
        write_test_todos(&dir)?;
        let mut names = fs::read_dir(dir.join("notes"))?
            .map(|res| res.map(|e| e.file_name().to_str().unwrap().to_string()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        let expected_names = vec![
            "33a25a20dcf8d607bcac45120f26ab158d5dbdd2",
            "560b05afe5e03eae9f8ad475b0b8b73ea6911272.todo",
            "63c5498f09d086fca6d870345350bfb210945790.todo",
            "b3942ad1c555625b7f60649fe50853830b6cdb04.todo",
        ];
        let mut expected_names: Vec<String> =
            expected_names.iter().map(|s| s.to_string()).collect();
        names.sort();
        expected_names.sort();

        remove_dir_all(dir)?;
        assert_eq!(names, expected_names);
        Ok(())
    }

    #[test]
    fn test_delete_todo() -> io::Result<()> {
        let dir = dir("test-delete-todo")?;
        let mut app = write_test_todos(&dir)?;
        app.delete_todo();
        app.write().expect("App writing failed");

        let names: io::Result<Vec<PathBuf>> = fs::read_dir(dir.join("notes"))
            .expect("Reading names failed")
            .map(|res| res.map(|e| e.path()))
            .collect();

        remove_dir_all(dir)?;
        assert!(names?.is_empty());
        Ok(())
    }

    #[test]
    fn test_remove_current_dependency() -> io::Result<()> {
        let dir = dir("test-remove-current-dependency")?;
        let mut app = write_test_todos(&dir)?;
        app.remove_current_dependent();
        app.write()?;

        let names: io::Result<Vec<PathBuf>> = fs::read_dir(dir.join("notes"))
            .unwrap()
            .map(|dir| dir.map(|entry| entry.path()))
            .collect();
        let string = fs::read_to_string(&dir.join("todo"))?;
        let expected_string = String::from("[0] Hello\n[0] Goodbye\n[0] Hello there\n");
        remove_dir_all(dir)?;
        assert!(names?.is_empty());
        assert_eq!(string, expected_string);
        Ok(())
    }

    #[test]
    fn test_remove_current_dependency_partial() -> io::Result<()> {
        let dir = dir("test-remove-current-dependency-partial")?;
        let mut app = write_test_todos(&dir)?;
        assert_eq!(app.index, 2);
        app.traverse_down();
        assert_eq!(app.index, 0);
        app.remove_current_dependent();
        app.write()?;

        let names: io::Result<Vec<PathBuf>> = fs::read_dir(dir.join("notes"))
            .unwrap()
            .map(|dir| dir.map(|entry| entry.path()))
            .collect();
        let expected = vec![PathBuf::from("test-remove-current-dependency-partial/notes/63c5498f09d086fca6d870345350bfb210945790.todo")];
        assert_eq!(names.unwrap(), expected);
        let string = fs::read_to_string(&dir.join("todo"))?;
        let expected_string = String::from("[0] Hello\n[0] Goodbye\n[0]>63c5498f09d086fca6d870345350bfb210945790.todo Hello there\n");
        remove_dir_all(dir)?;
        assert_eq!(string, expected_string);
        Ok(())
    }
}
