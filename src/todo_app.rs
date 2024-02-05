use std::io;
mod clipboard;
use clipboard::Clipboard;
mod todo_list;
pub use todo_list::TodoList;
use todo_list::todo::Todo;
use crate::Args;

pub struct App {
    clipboard: Clipboard,
    pub(super) todo_list: TodoList,
    index: usize,
    prior_indexes: Vec<usize>,
    pub changed:bool,
    pub(super) args: Args,
    // search:
    search_indexes: Vec<usize>,
    search_index: usize,
    last_query: String,
}

impl App {
    #[inline]
    pub fn new(args: Args) -> Self {
        App {
            todo_list: TodoList::read(&args.todo_path, !args.no_tree, true),
            clipboard: Clipboard::new(),
            index: 0,
            prior_indexes: vec![],
            changed: false,
            args,

            last_query: String::new(),
            search_indexes: vec![],
            search_index: 0,
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
        if let Some(mut todo) = self.mut_todo() {
            todo.schedule.add_days_to_done_date(1)
        }
    }

    #[inline]
    pub fn decrease_day_done(&mut self) {
        if let Some(mut todo) = self.mut_todo() {
            todo.schedule.add_days_to_done_date(-1)
        }
    }

    #[inline]
    pub fn show_done(&self) -> bool {
        self.args.show_done
    }

    #[inline]
    pub fn prepend(&mut self, message:String) {
        self.mut_current_list().prepend(Todo::default(message, 1));
        self.index = 0;
    }

    #[inline]
    pub fn append(&mut self, message:String) {
        self.mut_current_list().push(Todo::default(message, 0));
        self.index = self.current_list().undone.len()-1;
    }

    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn fix_done_undone(&mut self) {
        self.fix_dependency_done_undone();
        let show_done = self.args.show_done;
        let current_list = self.mut_current_list();
        current_list.fix_undone();
        if show_done {
            current_list.fix_done();
        }

        self.traverse_up_and_fix();
    }

    #[inline]
    fn fix_dependency_done_undone(&mut self) {
        let show_done = self.args.show_done;
        if let Some(todo) = self.mut_todo() {

            let dep_list = &mut todo.dependency.todo_list;

            dep_list.fix_undone();
            if show_done {
                dep_list.fix_done();
            }

        }

    }


    #[inline]
    fn traverse_up_and_fix(&mut self) {
        while self.only_undone_empty() && !self.is_root() {
            self.traverse_up();
            match self.mut_todo() {
                Some(todo) => {
                    todo.set_done(true)
                }
                _ => {}
            }
            self.mut_current_list().fix_undone();
            if self.args.show_done {
                self.mut_current_list().fix_done();
            }
        }
    }
    
    #[inline]
    pub fn search(&mut self, query:Option<String>) {
        if let Some(query) = query {
            self.last_query = query;
        }
        if self.last_query.is_empty() {
            return;
        }
        let mut todo_messages = self.current_list().undone.messages();
        if self.args.show_done {
            todo_messages.extend(self.current_list().done.messages());
        }
        self.search_indexes = Vec::new();

        for i in 0..todo_messages.len() {
            if todo_messages[i].to_lowercase().contains(self.last_query.to_lowercase().as_str()) {
                self.search_indexes.push(i);
            }
        }
    }

    #[inline]
    pub fn search_next_index(&mut self) {
        if self.search_indexes.is_empty() {
            return;
        }
        for index in &self.search_indexes {
            if *index > self.index{
                self.index = *index;
                return;
            }
        }

        self.index = self.search_indexes[0];
    }


    #[inline]
    pub fn toggle_show_done(&mut self) {
        self.args.show_done = !self.args.show_done;
        // while self.only_undone_empty() && !self.prior_indexes.is_empty() {
        //     self.traverse_up()
        // }
        self.search(None);
    }

    #[inline]
    pub fn search_next(&mut self) {
        if self.search_indexes.is_empty() {
            return;
        }
        if self.search_index+1 < self.search_indexes.len() {
            self.search_index+=1
        } else {
            self.search_index=0
        }
        self.index = self.search_indexes[self.search_index]
    }

    #[inline]
    pub fn search_prev(&mut self) {
        if self.search_indexes.is_empty() {
            return;
        }
        if self.search_index != 0 {
            self.search_index-=1
        } else {
            self.search_index=self.search_indexes.len()-1
        }
        self.index = self.search_indexes[self.search_index]
    }

    #[inline]
    pub fn toggle_current_done(&mut self) {
        let was_done = self.todo().unwrap().done();
        self.mut_todo().unwrap().toggle_done();
        self.fix_done_undone();
        if self.args.show_done {
            let index = if was_done {
                self.current_list().undone.len()-1
            } else {
                self.current_list().len()-1
            };
            self.index = self.mut_current_list().reorder(index);
        }
    }

    #[inline]
    pub fn read(&mut self) {
        self.changed = false;
        self.todo_list = TodoList::read(&self.args.todo_path, true, true);
    }

    #[inline]
    pub fn fix_index(&mut self) {
        let size = self.len();
        self.index = match size {
            0 => 0,
            _ => self.index.min(size-1),
        };
    }

    #[inline]
    pub fn parent(&self) -> Option<&Todo>{
        let mut list = &self.todo_list;
        let mut parent = None;
        for index in self.prior_indexes.iter() {
            parent = Some(&list[*index]);
            if let Some(todo_list) = &list[*index].dependency.todo_list() {
                list = todo_list
            } else {
                break
            }
        };
        parent
    }

    #[inline]
    pub fn increment(&mut self) {
        let size = self.len();
        if size == 0 {
            self.index = 0;
            return;
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
                    self.prior_indexes.push(self.index);
                    self.index = 0;
                }
                _ => {},
            }
        }
    }

    #[inline]
    pub fn traverse_up(&mut self) {
        if !self.is_root() {
            self.index = self.prior_indexes.remove(self.prior_indexes.len()-1);
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
        if self.args.show_done {
            self.current_list().is_empty()
        } else {
            self.is_undone_empty()
        }
    }

    #[inline]
    pub fn mut_todo(&mut self) -> Option<&mut Todo> {
        if self.is_todos_empty() {
            return None
        }
        let index = self.index.min(self.len() - 1);
        let size = self.len();

        if size <= index {
            return Some(&mut self.mut_current_list()[size - 1]);
        }

        Some(&mut self.mut_current_list()[index])
    }

    #[inline]
    pub fn cut_todo(&mut self) {
        if !self.is_todos_empty() {
            let index = self.index;
            let todo = self.mut_current_list().remove(index);
            let todo_string:String = (&todo).into();
            self.clipboard.set_text(todo_string);
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        if self.args.show_done {
            self.current_list().len()
        } else {
            self.current_list().undone.len()
        }
    }

    #[inline]
    pub fn mut_current_list(&mut self) -> &mut TodoList {
        self.changed = true;
        let is_root = self.is_root();
        let mut list = &mut self.todo_list;
        if  is_root{
            return list;
        }
        for index in self.prior_indexes.iter() {
            list = &mut list[*index].dependency.todo_list
        };
        list
    }

    #[inline]
    pub fn current_list(&self) -> &TodoList {
        let mut list = &self.todo_list;
        if self.is_root() {
            return list;
        }
        for index in self.prior_indexes.iter() {
            if let Some(todo_list) = &list[*index].dependency.todo_list() {
                list = todo_list
            } else {
                break
            }
        };
        list
    }


    #[inline]
    pub fn write(&mut self) -> io::Result<()> {
        self.changed = false;
        self.todo_list.write(&self.args.todo_path, true, self.is_tree())?;
        Ok(())
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.prior_indexes.is_empty()
    }

    
    #[inline]
    pub fn only_undone_empty(&self) -> bool {
        self.is_undone_empty() && !self.is_done_empty()
    }

    #[inline]
    pub fn toggle_current_daily(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.toggle_daily()
        }
    }

    #[inline]
    pub fn toggle_current_weekly(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.toggle_weekly()
        }
    }

    #[inline]
    pub fn is_undone_empty(&self) -> bool{
        self.current_list().undone.is_empty()
    }

    #[inline]
    pub fn is_done_empty(&self) -> bool{
        self.current_list().done.is_empty()
    }

    #[inline]
    pub fn set_current_priority(&mut self, priority:i8) {
        if let Some(todo) = self.mut_todo() {
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
            return Some(&current_list[size - 1]);
        }

        Some(&self.current_list()[index])
    }

    #[inline]
    pub fn reorder_current(&mut self) {
        let index = self.index;
        self.index = self.mut_current_list().reorder(index);
    }

    #[inline]
    pub fn delete_todo(&mut self) {
        if !self.is_todos_empty() {
            let index = self.index;
            self.mut_current_list().remove(index);
        }
    }

    #[inline]
    pub fn display(&self) -> Vec<String> {
        self.current_list().display(self.args.show_done)
    }

    #[inline]
    pub fn remove_current_dependent(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.remove_dependency();
        }
    }

    #[inline]
    pub fn add_dependency(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.add_todo_dependency();
        }
    }

    #[inline]
    pub fn edit_or_add_note(&mut self) {
        if self.is_tree() {
            if let Some(todo) = self.mut_todo() {
                todo.edit_note();
            }
        }
    }

    #[inline]
    pub fn decrease_current_priority(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.decrease_priority();
            self.reorder_current();
        }
    }
    
    #[inline]
    pub fn increase_current_priority(&mut self) {
        if let Some(todo) = self.mut_todo() {
            todo.increase_priority();
            self.reorder_current();
        }
    }

    #[inline]
    pub fn yank_todo(&mut self) {
        let todo_string:String = self.todo().unwrap().into();
        self.clipboard.set_text(todo_string);
    }

    #[inline]
    pub fn paste_todo(&mut self) {
        match Todo::try_from(self.clipboard.get_text()) {
            Ok(mut todo) => {
                let bottom = self.bottom();
                let todo_parent = TodoList::dependency_parent(&self.args.todo_path, true);
                todo.dependency.read(&todo_parent);
                let list = &mut self.mut_current_list();
                list.push(todo);
                self.index = list.reorder(bottom);
            },
            _ => {},
        };
    }

    #[inline]
    pub fn add_dependency_traverse_down(&mut self) {
        if self.is_tree() {
            if let Some(todo) = self.todo() {
                if todo.dependency.is_none() {
                    self.mut_todo().unwrap().add_todo_dependency();
                }
            }
            self.traverse_down()
        }
    }

    #[inline]
    pub fn print(&self) -> io::Result<()> {
        self.todo_list.print()
    }
}
