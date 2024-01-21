// vim:fileencoding=utf-8:foldmethod=marker
// std {{{
use std::io::{self, stdout};
// }}}
// lib {{{
use crossterm::{
    ExecutableCommand,
    terminal::{disable_raw_mode, LeaveAlternateScreen, enable_raw_mode, EnterAlternateScreen}
};
use ratatui::{prelude::*, widgets::*};
//}}}
// mod {{{
use crate::modules::potato::Potato;
mod app;
use app::App;
// }}}

pub fn default_block<'a, T>(title: T) -> Block<'a> 
where
    T: Into<Line<'a>>,
{
    Block::default().title(title).borders(Borders::ALL)
}


pub enum TodoWidget<'a> {
    List(ratatui::widgets::List<'a>),
    Paragraph(ratatui::widgets::Paragraph<'a>),
}

pub fn create_todo_widget(display_list:&Vec<String>, title:String) ->  TodoWidget {
    if display_list.len() == 0 {
        return TodoWidget::Paragraph(Paragraph::new("No todo.").block(default_block(title)))
    }
    return TodoWidget::List(List::new((*display_list).clone())
        .block(default_block(title))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true));

}

pub fn shutdown() -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(crossterm::cursor::Show)?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

pub fn startup() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Ok(())
}

#[inline]
pub fn redraw(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    terminal.clear()?;
    startup()?;
    Ok(())
}

#[inline]
pub fn run() -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut potato_module = Potato::new(None);
    let mut list_state = ListState::default();
    let mut app = App::new(&mut potato_module);

    loop {
        terminal.draw(|frame| {
            app.ui(frame, &mut list_state)
        })?;

        let should_redraw = app.update_return_should_redraw()?;
        if should_redraw {
            redraw(&mut terminal)?;
        }
    }
}
