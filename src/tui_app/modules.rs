pub mod potato;
use ratatui::widgets::Paragraph;

pub trait Module {
    fn update_time_ms(&self) -> u64;
    fn get_widget(&self) -> Paragraph<'_>;
    fn on_capital_c(&mut self);
    fn on_s(&mut self);
    fn on_capital_h(&mut self);
    fn on_capital_l(&mut self);
    fn on_f(&mut self);
    fn on_minus(&mut self);
    fn on_plus(&mut self);
    fn on_dot(&mut self);
    fn on_comma(&mut self);
    fn on_c(&mut self);
}
