use crate::flasher;
use crate::home;
use crate::root_app::{RootApp, Screen};
use crate::serial_monitor;
use crate::ui_theme as theme;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &RootApp) {
    match app.current_screen {
        Screen::Home => home::render(frame, &app.home),
        Screen::Serial => render_serial_protocol(frame, app.serial_selected),
        Screen::SerialMonitor => serial_monitor::render(frame, &app.serial_monitor),
        Screen::Flasher => flasher::render(frame, &app.flasher),
    }
}

fn render_serial_protocol(frame: &mut Frame, selected: usize) {
    let area = frame.area();
    let [header, body, notes, hint] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Fill(1),
        Constraint::Length(5),
        Constraint::Length(1),
    ])
    .areas(area);

    render_serial_header(frame, header);
    render_serial_workbench(frame, selected, body);
    render_serial_notes(frame, notes);

    frame.render_widget(
        Paragraph::new("  ↑↓:Select  Enter:Open  Esc:Protocol dashboard  q:Quit")
            .style(theme::muted_style()),
        hint,
    );
}

fn render_serial_header(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" SERIAL BUS WORKBENCH ", theme::selected_style()),
                Span::styled(" UART lane active ", theme::title_style()),
            ]),
            Line::from(
                " Monitor live traffic or switch the same bus into STM32 USART ISP flashing.",
            ),
        ])
        .alignment(Alignment::Center)
        .block(theme::active_panel_block(" Debug Assistant ")),
        area,
    );
}

fn render_serial_workbench(frame: &mut Frame, selected: usize, area: Rect) {
    let [nav, detail] =
        Layout::horizontal([Constraint::Length(28), Constraint::Fill(1)]).areas(area);
    render_serial_mode_select(frame, selected, nav);
    render_serial_function_detail(frame, selected, detail);
}

fn render_serial_mode_select(frame: &mut Frame, selected: usize, area: Rect) {
    let block = theme::panel_block(" Mode Select ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Fill(1),
    ])
    .split(inner);
    let items = [
        ("Serial Monitor", "RX/TX terminal", "Live traffic view"),
        ("USART ISP Flash", "ROM bootloader", "Firmware download"),
    ];
    for (idx, (title, subtitle, caption)) in items.iter().enumerate() {
        let style = if selected == idx {
            theme::selected_style()
        } else {
            Style::default().fg(Color::Gray)
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!(
                    "  {} {}",
                    if selected == idx { ">" } else { " " },
                    title
                )),
                Line::from(format!("    {}", subtitle)),
                Line::from(format!("    {}", caption)),
            ])
            .style(style),
            rows[idx],
        );
    }
}

fn render_serial_function_detail(frame: &mut Frame, selected: usize, area: Rect) {
    let (title, badge, lines): (&str, &str, &[&str]) = if selected == 0 {
        (
            " Selected Function — Serial Monitor ",
            " ONLINE DEBUG ",
            &[
                "Interactive receive/transmit terminal for UART devices.",
                "Display modes: ASCII, HEX and BOTH.",
                "Send history, newline suffix and HEX send mode are available.",
                "Use F2 inside the monitor to choose port and serial parameters.",
            ],
        )
    } else {
        (
            " Selected Function — USART ISP Flash ",
            " BOOTLOADER ",
            &[
                "Downloads STM32 firmware through the ROM USART bootloader.",
                "Supports BIN and HEX firmware files.",
                "Manual BOOT0/RESET or RTS/DTR assisted boot entry.",
                "If the monitor owns the same port, it is released and restored after flashing.",
            ],
        )
    };

    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut detail = Vec::with_capacity(lines.len() + 4);
    detail.push(Line::from(Span::styled(
        badge,
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    )));
    detail.push(Line::from(""));
    for line in lines {
        detail.push(Line::from(format!("  - {}", line)));
    }
    detail.push(Line::from(""));
    detail.push(Line::from(Span::styled(
        "  Press Enter to open this serial lane function.",
        theme::muted_style(),
    )));

    frame.render_widget(Paragraph::new(detail), inner);
}

fn render_serial_notes(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  SIGNAL NOTES  ", theme::title_style()),
                Span::styled("Shared UART resources", theme::muted_style()),
            ]),
            Line::from("  Serial Monitor keeps the port open for live debugging."),
            Line::from(
                "  USART ISP will temporarily release matching monitor sessions before flashing.",
            ),
        ])
        .block(
            Block::new()
                .title(" Bus Notes ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::MUTED)),
        ),
        area,
    );
}
