use ratatui::{
    layout::Rect,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct HelpPage {
    entries: HashMap<String, String>,
}

impl HelpPage {
    pub fn add_entry(&mut self, key: &str, value: &str) {
        self.entries.insert(key.to_string(), value.to_string());
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let mut content = String::new();
        for (key, value) in &self.entries {
            content.push_str(&format!("{}: {}\n", key, value));
        }

        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    }
}
