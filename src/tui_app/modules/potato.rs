use ratatui::widgets::Paragraph;
use std::io;
use std::process::{Command, Output};
use super::Module;
use super::super::default_block;

pub struct Potato<'a>{
    command: &'a str, 
    index: usize,
}

impl <'a> Module <'a> for Potato <'a> {
    #[inline]
    fn get_widget(&self) -> Paragraph<'a> {
        let args = vec!["+%m\n%t\n%p".to_string(), self.resolve_arg("1")];

        let time_str = match self.output(args) {
            Ok(output) => String::from_utf8(output.stdout).unwrap(),
            Err(_) => String::from("potctl command not found at path.")
        };

        Paragraph::new(time_str).block(default_block("Potato"))
    }

    #[inline]
    fn update_time_ms(&self) -> u64 {
        500
    }

    #[inline]
    fn on_c(&mut self) {
        self.quit()
    }

    #[inline]
    fn on_space(&mut self) {
        self.toggle_pause()
    }

    #[inline]
    fn on_s(&mut self) {
        self.skip();
    }

    #[inline]
    fn on_capital_h(&mut self) {
        self.increase_timer()
    }

    #[inline]
    fn on_capital_l(&mut self) {
        self.decrease_timer()
    }

    #[inline]
    fn on_r(&mut self) {
        self.restart()
    }

    #[inline]
    fn on_plus(&mut self) {
        self.increase_pomodoro();
    }

    #[inline]
    fn on_minus(&mut self) {
        self.decrease_pomodoro();
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

impl<'a> Potato<'a> {
    #[inline]
    pub fn new(command_name: Option<&'a str>) -> Self {
        let command = match command_name {
            Some(str) => str,
            None => "potctl",
        };
        Self { command, index: 0 }
    }

    #[inline]
    fn run(&self, args:Vec<String>) {
        let _ = Command::new(self.command).args(args).status();
    }

    #[inline]
    fn output(&self, args:Vec<String>) -> io::Result<Output>{
        Command::new(self.command).args(args).output()
    }

    #[inline]
    fn len(&self) -> usize {
        match self.output(vec![]) {
            Ok(output) => {
                String::from_utf8(output.stdout).unwrap().lines().count()-1
            }
            Err(_) => 0
        }
    }

    #[inline]
    fn resolve_arg(&self, arg:&str) -> String {
        format!("-{arg}{}", self.index)
    }

    #[inline]
    pub fn decrease_timer(&self) {
        self.run(vec![self.resolve_arg("d")])
    }

    #[inline]
    pub fn toggle_pause(&self) {
        self.run(vec![self.resolve_arg("t")])
    }

    #[inline]
    pub fn increase_timer(&self) {
        self.run(vec![self.resolve_arg("i")])
    }

    #[inline]
    pub fn increase_pomodoro(&self) {
        self.run(vec![self.resolve_arg("I")])
    }

    #[inline]
    pub fn decrease_pomodoro(&self) {
        self.run(vec![self.resolve_arg("D")])
    }

    #[inline]
    pub fn skip(&self) {
        self.run(vec![self.resolve_arg("s")])
    }

    #[inline]
    pub fn restart(&self) {
        self.run(vec![self.resolve_arg("r")])
    }

    #[inline]
    pub fn next(&mut self) {
        if self.index < self.len() - 1 {
            self.index += 1
        } else {
            self.index = 0
        }
    }

    #[inline]
    pub fn prev(&mut self) {
        if self.index > 0 {
            self.index -= 1
        } else {
            self.index = self.len() - 1;
        }
    }

    #[inline]
    pub fn quit(&mut self) {
        self.run(vec![self.resolve_arg("q")])
    }
}
