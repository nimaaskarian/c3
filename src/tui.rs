use ratatui::{prelude::*, widgets::*};

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
