use std::str::Lines;
use std::{io, path::PathBuf};
mod clipboard;
use clipboard::Clipboard;
mod todo_list;
mod todo;
mod search;
use search::Search;
use std::rc::Rc;
pub use todo::Todo;
use crate::Args;
use crate::fileio::{open_temp_editor, temp_path};

pub use self::todo::PriorityType;
pub use self::todo_list::TodoList;

#[derive(Clone)]
struct SearchPosition {
    tree_path: Vec<usize>,
    matching_indices: Vec<usize>,
}

pub type RestrictionFunction = Rc<dyn Fn(&Todo) -> bool>;
pub type Restriction = Option<RestrictionFunction>;
pub struct App {
    selected: Vec<usize>,
    clipboard: Clipboard,
    pub(super) todo_list: TodoList,
    index: usize,
    tree_path: Vec<usize>,
    changed:bool,
    pub(super) args: Args,
    removed_todos: Vec<Todo>,
    search: Search,
    tree_search_positions: Vec<SearchPosition>,
    last_query: String,
    x_index: usize,
    y_index: usize,
    restriction: Restriction,
}

impl App {
    #[inline]
    pub fn new(args: Args) -> Self {
        let todo_list = TodoList::read(&args.todo_path, !args.no_tree, true);
        let mut app = App {
            x_index: 0,
            y_index: 0,
            last_query: String::new(),
            tree_search_positions: vec![],
            removed_todos: vec![],
            selected: vec![],
            todo_list,
            clipboard: Clipboard::new(),
            index: 0,
            tree_path: vec![],
            changed: false,
            args,
            search: Search::new(),
            restriction: None,
        };
        app.update_show_done_restriction();
        app
    }

    pub fn restriction(&self) -> Restriction{
        self.restriction.clone()
    }

    #[inline]
    pub fn append_list_from_path(&mut self, path: PathBuf) {
        let todo_list = TodoList::read(&path, !self.args.no_tree, true);
        self.append_list(todo_list)
    }

    #[inline]
    pub fn append_list(&mut self, todo_list: TodoList) {
        self.current_list_mut().append_list(todo_list)
    }

    pub fn set_query_restriction(&mut self, query: String) {
        if self.show_done() {
            self.set_restriction(Rc::new(move |todo| todo.matches(query.as_str())))
        } else {
            self.set_restriction(Rc::new(move |todo| todo.matches(query.as_str()) && !todo.done()))
        }
    }

    pub fn do_commands_on_selected(&mut self) {
        for query in self.args.search_and_select.iter() {
            if self.args.delete_selected {
                self.changed = true;
                self.todo_list.set_todos(self.todo_list
                    .iter()
                    .filter(|todo| !todo.matches(query))
                    .cloned()
                    .collect());
                continue;
            }
            for todo in self.todo_list.iter_mut().filter(|todo| todo.matches(query)) {
                self.changed = true;
                if let Some(priority) = self.args.set_selected_priority {
                    todo.set_priority(priority as PriorityType);
                }
                if let Some(message) = self.args.set_selected_message.clone() {
                    todo.set_message(message);
                }
                if self.args.done_selected {
                    todo.set_done(true);
                }
            }
        }
        self.args.search_and_select = vec![];
    }

    fn traverse_parents_from_root(&mut self, callback: fn(&mut App, &TodoList, &[usize])) {
        self.todo_list.clone().traverse_tree(callback, None, self)
    }

    fn add_to_tree_positions(&mut self, list: &TodoList, prior_indices: &[usize]) {
        let mut matching_indices : Vec<usize> = vec![];
        for (i, todo) in list.todos(self.restriction.clone()).iter().enumerate() {
            if todo.matches(self.last_query.as_str()) {
                matching_indices.push(i)
            }
        }
        if !matching_indices.is_empty() {
            self.tree_search_positions.push(SearchPosition {
                tree_path: prior_indices.to_vec(),
                matching_indices,
            })
        }
    }

    pub fn tree_search(&mut self, query:Option<String>) {
        if let Some(query) = query {
            self.last_query = query;
        }
        self.tree_search_positions = vec![];
        self.y_index = 0;
        self.x_index = 0;
        if self.last_query.is_empty() {
            return;
        }
        let before_position = SearchPosition {
            tree_path: self.tree_path.clone(),
            matching_indices: vec![self.index],
        };
        self.tree_search_positions.push(before_position);
        self.traverse_parents_from_root(Self::add_to_tree_positions);
        self.search_next();
    }

    pub fn batch_editor_messages(&mut self) {
        let restriction = self.restriction().clone();
        let content = self.current_list().messages(restriction).join("\n");
        let new_messages = open_temp_editor(Some(&content),temp_path("messages")).unwrap();
        let mut new_messages = new_messages.lines();
        self.batch_edit_current_list(new_messages)
    }

    #[inline(always)]
    fn batch_edit_current_list(&mut self, mut messages: Lines<'_>) {
        let mut changed = false;
        if let Some(restriction) = self.restriction.clone() {
            for todo in self.current_list_mut().todos.iter_mut().filter(|todo| restriction(todo)) {
                if Self::batch_edit_helper(todo, messages.next()) {
                    changed = true;
                }
            }
        } else {
            for todo in self.current_list_mut().todos.iter_mut() {
                if Self::batch_edit_helper(todo, messages.next()) {
                    changed = true;
                }
            }
        }
        while let Some(message) = messages.next() {
            changed = true;
            self.append(String::from(message))
        }
        self.changed = changed;
    }

    #[inline(always)]
    fn batch_edit_helper(todo: &mut Todo, message: Option<&str>) -> bool {
        if let Some(message) = message {
            let message = String::from(message);
            if todo.message == message {
                return false
            }
            todo.set_message(message);
            return true
        } 
        false
    }

    pub fn print_searched(&mut self) {
        for position in self.tree_search_positions.iter() {
            self.tree_path = position.tree_path.clone();
            let list = self.current_list();
            for index in position.matching_indices.clone() {
                println!("{}",list.index(index,self.restriction.clone()).display(&self.args.display_args));
            }
        }
    }

    #[inline]
    pub fn is_tree(&self) -> bool {
        !self.args.no_tree
    }

    #[inline]
    pub fn is_changed(&self) -> bool {
        self.changed
    }

    #[inline]
    pub fn increase_day_done(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.schedule.add_days_to_date(-1)
        }
    }

    #[inline]
    pub fn decrease_day_done(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.schedule.add_days_to_date(1)
        }
    }

    #[inline]
    pub fn prepend(&mut self, message:String) {
        self.current_list_mut().prepend(Todo::default(message, 1));
        self.go_top();
    }

    #[inline]
    pub fn append(&mut self, message:String) {
        self.current_list_mut().push(Todo::default(message, 0));
        self.index = self.current_list_mut().reorder_last();
    }

    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn search(&mut self, query:Option<String>) {
        let todo_messages = self.current_list().messages(self.restriction.clone());
        self.search.search(query, todo_messages);
    }

    #[inline]
    pub fn search_init(&mut self) {
        if let Some(index) = self.search.first_greater_than(self.index) {
            self.index = index;
        }
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
        if self.tree_search_positions.is_empty() {
            if let Some(index) = self.search.next() {
                self.index = index
            }
        } else {
            let x_size = self.tree_search_positions.len();
            let y_size = self.tree_search_positions[self.x_index].matching_indices.len();
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
        let item = self.tree_search_positions[self.x_index].clone();
        self.tree_path = item.tree_path;
        self.index = item.matching_indices[self.y_index];
    }

    #[inline]
    pub fn search_prev(&mut self) {
        if let Some(index) = self.search.prev() {
            self.index = index
        }
    }

    #[inline]
    pub fn toggle_current_done(&mut self) {
        let index = self.index;
        self.todo_mut().unwrap().toggle_done();
        if self.show_done() {
            self.index = self.current_list_mut().reorder(index);
        } else {
            self.current_list_mut().sort();
        }
    }

    #[inline]
    pub fn read(&mut self) {
        self.changed = false;
        self.todo_list = TodoList::read(&self.args.todo_path, true, true);
    }

    #[inline]
    pub fn fix_index(&mut self) {
        let size = self.current_list().len(self.restriction.clone());
        self.index = match size {
            0 => 0,
            _ => self.index.min(size-1),
        }
    }

    #[inline]
    pub fn parent(&mut self) -> Option<&Todo>{
        let mut list = &self.todo_list;
        let mut parent = None;
        for index in self.tree_path.clone() {
            parent = Some(&list.todos[index]);
            if let Some(todo_list) = list.todos[index].dependency.todo_list() {
                list = todo_list
            } else {
                break
            }
        }
        parent
    }

    #[inline]
    pub fn increment(&mut self) {
        let size = self.len();
        if size == 0 {
            return self.go_top();
        };
        if self.index != size - 1 {
            self.index += 1
        } else {
            self.go_top()
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
    pub fn go_top(&mut self) {
        self.index = 0;
    }

    #[inline]
    pub fn traverse_down(&mut self) {
        if self.is_tree() {
            match self.todo() {
                Some(todo) if todo.dependency.is_list() => {
                    let index = self.index;
                    let restriction = self.restriction.clone();
                    let true_index = self.current_list().true_position_in_list(index, restriction);
                    self.tree_path.push(true_index);
                    self.go_top();
                    self.search(None);
                }
                _ => {},
            }
        }
    }

    #[inline]
    pub fn traverse_up(&mut self) {
        if let Some(index) = self.tree_path.pop() {
            self.index = index;
            self.search(None);
        }
    }

    #[inline]
    pub fn go_bottom(&mut self) {
        self.index = self.bottom();
    }

    #[inline]
    pub fn bottom(&self) -> usize {
        match self.len() {
            0=>0,
            length=>length-1,
        }
    }

    #[inline]
    pub fn is_todos_empty(&self) -> bool{
        if self.show_done() {
            self.current_list().is_empty(self.restriction.clone())
        } else {
            self.is_undone_empty()
        }
    }

    #[inline]
    pub fn todo_mut(&mut self) -> Option<&mut Todo> {
        if self.is_todos_empty() {
            return None
        }
        let index = self.index.min(self.len() - 1);
        let size = self.len();
        let res_cloned = self.restriction.clone();

        if size <= index {
            return Some(self.current_list_mut().index_mut(size - 1, res_cloned));
        }

        Some(self.current_list_mut().index_mut(index, res_cloned))
    }

    #[inline]
    pub fn cut_todo(&mut self) {
        let restriction = self.restriction.clone();
        if !self.is_todos_empty() {
            let index = self.index;
            let todo = self.current_list_mut().cut(index, restriction);
            let todo_string:String = (&todo).into();
            self.clipboard.set_text(todo_string);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.current_list().len(self.restriction.clone())
    }

    #[inline]
    pub fn current_list_mut(&mut self) -> &mut TodoList {
        self.changed = true;
        let is_root = self.is_root();
        let mut list = &mut self.todo_list;
        if  is_root{
            return list;
        }
        for index in self.tree_path.clone() {
            list = &mut list.todos[index].dependency.todo_list
        };
        list
    }

    #[inline]
    pub fn current_list(&self) -> &TodoList {
        let mut list = &self.todo_list;
        if self.is_root() {
            return list;
        }
        for index in self.tree_path.clone() {
            if let Some(todo_list) = &list.todos[index].dependency.todo_list() {
                list = todo_list
            } else {
                break
            }
        };
        list
    }

    #[inline]
    pub fn handle_removed_todo_dependency_files(&mut self, dependency_path:&PathBuf) {
        for todo in &mut self.removed_todos {
            let _ = todo.delete_dependency_file(dependency_path);
        }
        self.removed_todos = vec![];
    }

    #[inline]
    pub fn write(&mut self) -> io::Result<bool> {
        if self.changed {
            self.changed = false;
            let dependency_path = self.todo_list.write(&self.args.todo_path, true)?;
            self.handle_removed_todo_dependency_files(&dependency_path);
            self.todo_list.delete_removed_dependent_files(&dependency_path)?;
            if self.is_tree() {
                self.todo_list.write_dependencies(&dependency_path)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
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
            todo.toggle_daily()
        }
    }

    #[inline]
    pub fn toggle_current_weekly(&mut self) {
        if let Some(todo) = self.todo_mut() {
            todo.toggle_weekly()
        }
    }

    #[inline]
    pub fn is_undone_empty(&self) -> bool{
        self.current_list().is_empty(self.restriction.clone())
    }

    #[inline]
    pub fn is_done_empty(&self) -> bool{
        self.current_list().is_empty(self.restriction.clone())
    }

    #[inline(always)]
    pub fn unset_restriction(&mut self) {
        self.restriction = None;
    }

    #[inline(always)]
    pub fn set_restriction(&mut self, restriction: RestrictionFunction) {
        self.restriction = Some(restriction);
        self.fix_index();
    }

    #[inline]
    pub fn set_priority_restriction(&mut self, priority:PriorityType) {
        self.args.display_args.show_done = true;
        self.set_restriction(Rc::new(move |todo| todo.priority() == priority))
    }

    #[inline]
    pub fn set_priority_limit_no_done(&mut self, priority:PriorityType) {
        self.args.display_args.show_done = false;
        self.set_restriction(Rc::new(move |todo| todo.priority() == priority && !todo.done()))
    }

    #[inline]
    pub fn set_current_priority(&mut self, priority:PriorityType) {
        if let Some(todo) = self.todo_mut() {
            todo.set_priority(priority);
            self.reorder_current();
        }
    }

    #[inline]
    pub fn get_message(&mut self) -> Option<String> {
        if let Some(todo) = self.todo() {
            return Some(todo.message.clone())
        };
        None
    }

    #[inline]
    pub fn todo(&self) -> Option<&Todo> {
        if self.is_todos_empty() {
            return None
        }

        let current_list = self.current_list();
        let index = self.index.min(self.len() - 1);
        let size = self.len();

        if size <= index {
            return Some(&current_list.index(size - 1, self.restriction.clone()));
        }

        Some(&self.current_list().index(index, self.restriction.clone()))
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
            let todo = self.current_list_mut().cut(index, restriction);
            self.removed_todos.push(todo);
        }
    }

    #[inline]
    pub fn display_current(&self) -> Vec<String> {
        self.display_list(self.current_list())
    }

    #[inline]
    pub fn display_list(&self, todo_list: &TodoList) -> Vec<String> {
        todo_list.display(&self.args.display_args, self.restriction.clone())
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
            if let Some(todo) = self.todo_mut() {
                let _ = todo.edit_note();
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
            let todo_string:String = todo.into();
            self.clipboard.set_text(todo_string);
        }
    }

    #[inline]
    pub fn paste_todo(&mut self) {
        let todos_count = self.len();
        match Todo::try_from(self.clipboard.get_text()) {
            Ok(mut todo) => {
                let todo_parent = TodoList::dependency_parent(&self.args.todo_path, true);
                let _ = todo.dependency.read(&todo_parent);
                let bottom = self.bottom()+1;
                let list = &mut self.current_list_mut();
                list.push(todo);
                if todos_count != 0 {
                    self.index = list.reorder(bottom);
                }
            },
            _ => {},
        };
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
    pub fn print(&self) -> io::Result<()> {
        self.todo_list.print()
    }

    #[inline]
    pub fn print_selected(&self) {
        for index in self.selected.clone() {
            println!("{}", self.todo_list.index(index, self.restriction.clone()).display(&self.args.display_args));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, remove_dir_all};

    use clap::Parser;
    use crate::Args;

    use super::*;

    fn dir(dir_name: &str) -> io::Result<PathBuf >{
        let path = PathBuf::from(dir_name);
        fs::create_dir_all(path.join("notes"))?;
        Ok(path)
    }

    fn write_test_todos(dir: &PathBuf) -> io::Result<App>{
        let mut args = Args::parse();
        fs::create_dir_all(dir.join("notes"))?;
        args.todo_path = dir.join("todo");
        let mut app = App::new(args);
        app.append(String::from("Hello"));
        let dependencies = vec!["Is there anybody outthere?", "Just nod if you can here me", "Is there anyone home"];
        for dependency in dependencies {
            app.add_dependency_traverse_down();
            app.append(String::from(dependency));
        }
        for _ in 0..3 {
            app.traverse_up();
        }
        app.write()?;
        Ok(app)
    }

    #[test]
    fn test_is_changed() -> io::Result<()>{
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
    fn test_set_restrictions_done() -> io::Result<()>{
        let dir = dir("test-set-restrictions-done")?;
        let mut app = write_test_todos(&dir)?;
        app.toggle_current_done();
        assert_eq!(app.len(), 0);
        app.toggle_show_done();
        assert_eq!(app.len(), 1);
        app.toggle_show_done();
        assert_eq!(app.len(), 0);
        remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn test_set_restrictions_query() -> io::Result<()>{
        let dir = dir("test-set-restrictions-query")?;
        let mut app = write_test_todos(&dir)?;
        assert_eq!(app.len(), 1);
        app.set_query_restriction(String::from("hello"));
        assert_eq!(app.len(), 1);
        app.unset_restriction();
        assert_eq!(app.len(), 1);
        remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn test_set_restrictions_priority() -> io::Result<()>{
        let dir = dir("test-set-restrictions-priority")?;
        let mut app = write_test_todos(&dir)?;
        app.set_current_priority(2);
        assert_eq!(app.len(), 1);
        app.set_priority_restriction(2);
        assert_eq!(app.len(), 1);
        app.set_priority_restriction(0);
        assert_eq!(app.len(), 0);
        remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn test_tree_search() -> io::Result<()>{
        let dir = dir("test-tree-search")?;
        let mut app = write_test_todos(&dir)?;
        remove_dir_all(dir)?;
        let query = String::from("nod");
        app.tree_search(Some(query));
        let position = &app.tree_search_positions[1];
        assert_eq!(position.tree_path,vec![0,0]);
        assert_eq!(position.matching_indices,vec![0]);
        Ok(())
    }

    #[test]
    fn test_write() -> io::Result<()> {
        let dir = dir("test-write")?;
        write_test_todos(&dir)?;
        let mut names = fs::read_dir(dir.join("notes"))?
            .map(|res| res.map(|e| e.file_name().to_str().unwrap().to_string()))
            .collect::<Result<Vec<_>, io::Error>>()?;

        let expected_names = vec!["275549796be6d9a9c6b45d71df4714bfd934c0ba.todo", "560b05afe5e03eae9f8ad475b0b8b73ea6911272.todo", "b3942ad1c555625b7f60649fe50853830b6cdb04.todo"];
        let mut expected_names : Vec<String> = expected_names.iter()
            .map(|s|s.to_string()).collect();
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

        let names : io::Result<Vec<PathBuf>> = fs::read_dir(dir.join("notes")).expect("Reading names failed")
            .map(|res| res.map(|e|e.path())).collect();

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

        let names : io::Result<Vec<PathBuf>> = match fs::read_dir(dir.join("notes")) {
            Ok(value) => value.map(|res| res.map(|e|e.path())).collect(),
            _ => Ok(vec![]),
        };
        let string = fs::read_to_string(&dir.join("todo"))?;
        let expected_string = String::from("[0] Hello\n");
        remove_dir_all(dir)?;
        assert!(names?.is_empty());
        assert_eq!(string, expected_string);
        Ok(())
    }
}
