pub mod potato;
use ratatui::widgets::Paragraph;

pub trait Module <'a>{
    fn update_time_ms(&self) -> u64;
    fn get_widget(&self) -> Paragraph<'a>;
    fn on_space(&mut self);
    fn on_s(&mut self);
    fn on_capital_h(&mut self);
    fn on_capital_l(&mut self);
    fn on_r(&mut self);
    fn on_minus(&mut self);
    fn on_plus(&mut self);
    fn on_dot(&mut self);
    fn on_comma(&mut self);
    fn on_c(&mut self);
}
