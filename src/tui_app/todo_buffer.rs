use c3::todo_app::Todo;

#[derive(Default)]
pub struct TodoBuffer {
    buffer: Option<Todo>,
}

impl TodoBuffer {
    #[inline]
    pub fn yank<T>(&mut self, todo: T) 
    where 
    T: Into<Option<Todo>>,
    { 
        if let Some(todo) = todo.into() {
            self.buffer = Some(todo);
        }
    }

    #[inline]
    pub fn get(&self) -> Option<Todo> {
        self.buffer.clone()
    }
}

