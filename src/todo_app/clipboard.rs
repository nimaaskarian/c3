#[derive(Debug)]
pub struct Clipboard {
    buffer: String,
}

impl Clipboard {
    pub fn new() -> Self {
        Clipboard {
            buffer: String::new(),
        }
    }

    pub fn get_text(&self) -> &str {
        &self.buffer
    }

    pub fn set_text(&mut self, text: String) {
        self.buffer = text;
    }
}
