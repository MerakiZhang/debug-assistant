use super::state::{HomeState, MENU_ITEMS};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_big_text::{BigText, PixelSize};

pub fn render(frame: &mut Frame, state: &HomeState) {
    let area = frame.area();

    let [title_area, menu_area, footer_area] = Layout::vertical([
        Constraint::Percentage(45),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // BigText title: "Debug" / "Assistant" stacked, centered
    let big = BigText::builder()
        .pixel_size(PixelSize::HalfHeight)
        .centered()
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .lines(vec![Line::from("Debug"), Line::from("Assistant")])
        .build();
    frame.render_widget(big, title_area);

    // Centered menu box
    let [_, center_col, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(32),
        Constraint::Fill(1),
    ])
    .areas(menu_area);

    let [_, items_area, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(MENU_ITEMS.len() as u16 + 2),
        Constraint::Fill(1),
    ])
    .areas(center_col);

    let menu_block = Block::new()
        .title(" Select Tool ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let menu_inner = menu_block.inner(items_area);
    frame.render_widget(menu_block, items_area);

    let item_constraints: Vec<Constraint> =
        MENU_ITEMS.iter().map(|_| Constraint::Length(1)).collect();
    let item_rows = Layout::vertical(item_constraints).split(menu_inner);

    for (i, item) in MENU_ITEMS.iter().enumerate() {
        let (style, prefix) = if i == state.selected {
            (
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
                " ► ",
            )
        } else {
            (Style::default().fg(Color::Gray), "   ")
        };
        frame.render_widget(
            Paragraph::new(format!("{}{}", prefix, item)).style(style),
            item_rows[i],
        );
    }

    // Version — bottom right
    frame.render_widget(
        Paragraph::new(" v0.1.1 ")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right),
        footer_area,
    );
}
