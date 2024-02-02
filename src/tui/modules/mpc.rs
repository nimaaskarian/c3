use ratatui::widgets::Paragraph;
use std::io;
use std::process::{Command, Output};
use crate::tui::modules::Module;
use crate::tui::default_block;

pub struct MusicPlayerClient<'a>{
    command: &'a str, 
}

impl <'a> Module <'a> for MusicPlayerClient <'a> {
    #[inline]
    fn get_widget(&self) -> Paragraph<'a> {
        let str = match self.output(vec![]) {
            Ok(output) => String::from_utf8(output.stdout).unwrap(),
            Err(_) => String::from("mpc command not found at path.")
        };

        Paragraph::new(str).block(default_block("MPC"))
    }

    #[inline]
    fn update_time_ms(&self) -> u64 {
        500
    }

    #[inline]
    fn on_c(&mut self) {
        self.toggle_pause()
    }

    #[inline]
    fn on_space(&mut self) {
        self.toggle_pause()
    }

    #[inline]
    fn on_s(&mut self) {
        self.stop();
    }

    #[inline]
    fn on_capital_h(&mut self) {
        self.seek_backward()
    }

    #[inline]
    fn on_capital_l(&mut self) {
        self.seek_forward()
    }

    #[inline]
    fn on_r(&mut self) {
    }

    #[inline]
    fn on_plus(&mut self) {
        self.vol_up();
    }

    #[inline]
    fn on_minus(&mut self) {
        self.vol_down();
    }

    #[inline]
    fn on_dot(&mut self) {
        self.next();
    }

    #[inline]
    fn on_comma(&mut self) {
        self.prev();
    }
}

impl<'a> MusicPlayerClient<'a> {
    #[inline]
    pub fn new(command_name: Option<&'a str>) -> Self {
        let command = match command_name {
            Some(str) => str,
            None => "mpc",
        };
        Self { command }
    }

    #[inline]
    fn run(&self, args:Vec<&str>) {
        let _ = Command::new(self.command).args(args).status();
    }

    #[inline]
    fn output(&self, args:Vec<&str>) -> io::Result<Output>{
        Command::new(self.command).args(args).output()
    }

    pub fn next(&self) {
        self.output(vec!["next"]);
    }

    pub fn prev(&self) {
        self.output(vec!["prev"]);
    }

    pub fn stop(&self) {
        self.output(vec!["stop"]);
    }

    pub fn seek_forward(&self) {
        self.output(vec!["seek", "+5"]);
    }

    pub fn seek_backward(&self) {
        self.output(vec!["seek", "-5"]);
    }

    pub fn toggle_pause(&self) {
        self.output(vec!["toggle"]);
    }

    pub fn vol_up(&self) {
        self.output(vec!["volume", "+1"]);
    }

    pub fn vol_down(&self) {
        self.output(vec!["volume", "-1"]);
    }
}
