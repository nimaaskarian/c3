use scanf::sscanf;
mod note;
use note::{Note, sha1};

use self::note::NoteError;
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Todo {
    message: String,
    note: String,
    priority: i8,
    pub(crate) done:bool,
}

impl Into<String> for &Todo {
    fn into(self) -> String{
        let done_str = if self.done {"-"} else {""};
        let note_str = match self.note.as_str() {
            "" => String::new(),
            _ => format!(">{}", self.note),
        };
        format!("[{done_str}{}]{note_str} {}", self.priority, self.message)
    }
}

#[derive(Debug, PartialEq)]
pub enum TodoError {
    ReadFailed,
    NoteEmpty
}

impl TryFrom<String> for Todo {
    type Error = TodoError;

    fn try_from(s:String) -> Result<Todo, TodoError>{
        Todo::try_from(s.as_str())
    }
}

impl TryFrom<&str> for Todo {
    type Error = TodoError;

    fn try_from(s:&str) -> Result<Todo, TodoError>{
        let mut message = String::new();
        let mut note = String::new();
        let mut priority_string:String = String::new();

        if sscanf!(s,"[{}]>{} {}", priority_string, note, message).is_err() {
            if sscanf!(s,"[{}] {}", priority_string, message).is_err() {
                return Err(TodoError::ReadFailed);
            }
        }
        let done = priority_string.chars().nth(0).unwrap() == '-';
        let mut priority:i8 = priority_string.parse().unwrap();

        if done {
            priority*=-1;
        }
        Ok(Todo {
            message,
            note,
            priority,
            done,
        })
    }
}

impl Todo {
    pub fn new(message:String, priority:i8) -> Self {
        Todo {
            note: String::new(),
            message,
            priority: Todo::fixed_priority(priority),
            done: false,
        }
    }

    pub fn add_note(&mut self)-> Result<(), NoteError>{
        let note = Note::from_editor()?;

        self.note = note.hash();
        note.save().expect("Note saving failed");
        Ok(())
    }

    pub fn hash(&self) -> String{
        sha1(&self.as_string())
    }

    pub fn toggle_done(&mut self) {
        self.done = !self.done;
    }

    pub fn add_priority(&mut self, add:i8) {
        self.priority += add;
        self.fix_priority();
    }

    pub fn set_priority(&mut self, add:i8) {
        self.priority = add;
        self.fix_priority();
    }

    fn fix_priority(&mut self) {
        self.priority = Todo::fixed_priority(self.priority)
    }

    #[inline(always)]
    pub fn comparison_priority(&self) -> i8{
        if self.priority == 0 {10} else {self.priority}
    }

    fn fixed_priority(priority: i8) -> i8 {
        match priority {
            ..=0 => 0,
            9.. => 9,
            _ => priority
        }
    }

    pub fn to_display_string(&self) -> String {
        unimplemented!()
    }

    pub fn as_string(&self) -> String{
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_todo() {
        let todo = Todo::new("Buy groceries".to_string(), 5);
        assert_eq!(todo.message, "Buy groceries");
        assert_eq!(todo.priority, 5);
        assert_eq!(todo.done, false);
    }

    #[test]
    fn test_toggle_done() {
        let mut todo = Todo::new("Buy groceries".to_string(), 5);
        assert_eq!(todo.done, false);

        todo.toggle_done();
        assert_eq!(todo.done, true);

        todo.toggle_done();
        assert_eq!(todo.done, false);
    }

    #[test]
    fn test_add_priority() {
        let mut todo = Todo::new("Buy groceries".to_string(), 5);
        assert_eq!(todo.priority, 5);

        todo.add_priority(3);
        assert_eq!(todo.priority, 8);

        todo.add_priority(-2);
        assert_eq!(todo.priority, 6);

        todo.add_priority(-10);
        assert_eq!(todo.priority, 0);

        todo.add_priority(10);
        assert_eq!(todo.priority, 9);
    }

    #[test]
    fn test_set_priority() {
        let mut todo = Todo::new("Buy groceries".to_string(), 5);
        assert_eq!(todo.priority, 5);

        todo.set_priority(8);
        assert_eq!(todo.priority, 8);

        todo.set_priority(2);
        assert_eq!(todo.priority, 2);
    }

    #[test]
    fn test_fix_priority() {
       let mut todo = Todo::new("Buy groceries".to_string(), -1); 
       todo.fix_priority();
       assert_eq!(todo.priority, 0);

       let mut todo = Todo::new("Buy groceries".to_string(), 10); 
       todo.fix_priority();
       assert_eq!(todo.priority, 9);

       let mut todo = Todo::new("Buy groceries".to_string(), 4); 
       todo.fix_priority();
       assert_eq!(todo.priority, 4);  
   }
    #[test]
    fn test_into_string() {
        let mut todo = Todo {
            done: true,
            note: String::from("Note very gud"),
            priority: 1,
            message: String::from("Important job :D"),
        };

        let result: String = todo.as_string();

        assert_eq!(result, "[-1]>Note very gud Important job :D");

        todo.toggle_done();
        let result: String = todo.as_string();
        assert_eq!(result, "[1]>Note very gud Important job :D");
    }
     #[test]
    fn test_try_from_string() {
        let input = String::from("[2]>Note Message");

        let result = Todo::try_from(input).unwrap();

        assert_eq!(
            result,
            Todo {
                done: false,
                note: String::from("Note"),
                priority: 2,
                message: String::from("Message"),
            }
        );
    }

    #[test]
    fn test_try_from_string_with_invalid_input() {
        let input = String::from("[invalid] Invalid Message");

        let result = Todo::try_from(input);

        assert_eq!(result, Err(TodoError::ReadFailed));
    }
}
