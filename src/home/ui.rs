use super::state::{HomeState, MENU_ITEMS};
use crate::ui_theme as theme;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, state: &HomeState) {
    let area = frame.area();
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    render_header(frame, header);
    render_dashboard(frame, state, body);
    frame.render_widget(
        Paragraph::new("  ↑↓:Select protocol  Enter:Open  q:Quit").style(theme::muted_style()),
        footer,
    );
}

fn render_header(frame: &mut Frame, area: Rect) {
    let version = format!(" v{} ", env!("CARGO_PKG_VERSION"));
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" DEBUG ASSISTANT ", theme::selected_style()),
            Span::styled(" MCU protocol control deck ", theme::title_style()),
        ]),
        Line::from(" Serial, JTAG and SWD workflows are grouped by physical/debug protocol."),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::new()
            .title(version)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT)),
    );
    frame.render_widget(header, area);
}

fn render_dashboard(frame: &mut Frame, state: &HomeState, area: Rect) {
    let [left, right] =
        Layout::horizontal([Constraint::Length(34), Constraint::Fill(1)]).areas(area);
    render_protocol_nav(frame, state, left);
    render_protocol_detail(frame, state.selected, right);
}

fn render_protocol_nav(frame: &mut Frame, state: &HomeState, area: Rect) {
    let block = theme::panel_block(" Protocol Bay ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .split(inner);

    for (idx, item) in MENU_ITEMS.iter().enumerate() {
        let selected = state.selected == idx;
        let style = if selected {
            theme::selected_style()
        } else if *item == "Quit" {
            theme::muted_style()
        } else {
            Style::default().fg(Color::Gray)
        };
        let status = match *item {
            "Serial" => "UART terminal + ISP",
            "JTAG" => "Probe flash lane",
            "SWD" => "Probe flash lane",
            _ => "Leave console",
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!("  {} {}", if selected { ">" } else { " " }, item)),
                Line::from(format!("    {}", status)),
            ])
            .style(style),
            rows[idx],
        );
    }
}

fn render_protocol_detail(frame: &mut Frame, selected: usize, area: Rect) {
    let (title, color, lines): (&str, Color, &[&str]) = match selected {
        0 => (
            " Serial Workbench ",
            theme::SUCCESS,
            &[
                "RX/TX terminal with ASCII, HEX and BOTH display modes",
                "Configurable baud, data bits, stop bits, parity and flow control",
                "USART ISP flashing for STM32 ROM bootloader",
                "Serial monitor is released before ISP flashing and restored after completion",
            ],
        ),
        1 => (
            " JTAG Flash Station ",
            theme::WARNING,
            &[
                "Probe enumeration and target attach through probe-rs",
                "Manual chip name input for STM32 targets",
                "BIN and HEX firmware download",
                "Progress and operation log during flashing",
            ],
        ),
        2 => (
            " SWD Flash Station ",
            Color::Magenta,
            &[
                "Probe selection with SWD speed presets",
                "Normal or Under Reset attach mode",
                "Chip presets, BIN base address, verify and reset-run options",
                "Progress and operation log during flashing",
            ],
        ),
        _ => (
            " Shutdown ",
            theme::DANGER,
            &["Close the terminal UI and return to the shell."],
        ),
    };

    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut rendered = Vec::with_capacity(lines.len() + 4);
    rendered.push(Line::from(Span::styled(
        "  ONLINE",
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )));
    rendered.push(Line::from(""));
    for line in lines {
        rendered.push(Line::from(format!("  - {}", line)));
    }
    rendered.push(Line::from(""));
    rendered.push(Line::from(Span::styled(
        "  Press Enter to open this protocol lane.",
        theme::muted_style(),
    )));

    frame.render_widget(Paragraph::new(rendered), inner);
}
