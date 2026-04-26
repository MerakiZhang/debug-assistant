use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
};

pub const ACCENT: Color = Color::Cyan;
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const DANGER: Color = Color::Red;
pub const MUTED: Color = Color::DarkGray;

pub fn title_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn selected_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn panel_block(title: &'static str) -> Block<'static> {
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
}

pub fn active_panel_block(title: &'static str) -> Block<'static> {
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
}

pub fn status_panel_block(title: &'static str, color: Color) -> Block<'static> {
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
}
