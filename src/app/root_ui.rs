use crate::app::root::{RootApp, Screen};
use crate::features::home;
use crate::features::protocols::flashing as flasher;
use crate::features::protocols::uart::monitor;
use crate::ui::theme;
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
        Screen::Uart => render_serial_protocol(frame, app.serial_selected),
        Screen::Jtag => render_probe_protocol(
            frame,
            ProbeProtocolView {
                protocol: "JTAG",
                title: " JTAG PROTOCOL WORKBENCH ",
                badge: " PROBE FLASH ",
                color: theme::WARNING,
                summary:
                    " Attach through a debug probe and flash targets over the JTAG wire protocol.",
                details: &[
                    "Probe enumeration and target attach through probe-rs.",
                    "Configurable probe speed, verify and reset-run options.",
                    "BIN and HEX firmware download with progress logging.",
                    "More JTAG tools can be added under this protocol lane later.",
                ],
            },
        ),
        Screen::Swd => render_probe_protocol(
            frame,
            ProbeProtocolView {
                protocol: "SWD",
                title: " SWD PROTOCOL WORKBENCH ",
                badge: " PROBE FLASH ",
                color: Color::Magenta,
                summary:
                    " Attach through a debug probe and flash targets over the SWD wire protocol.",
                details: &[
                    "Normal and Under Reset attach modes.",
                    "Configurable SWD speed, verify and reset-run options.",
                    "Chip presets plus manual target name input.",
                    "More SWD tools can be added under this protocol lane later.",
                ],
            },
        ),
        Screen::I2c => render_placeholder_protocol(
            frame,
            PlaceholderProtocolView {
                protocol: "I2C",
                title: " I2C PROTOCOL WORKBENCH ",
                color: Color::Blue,
                summary: " I2C tools will live here once the concrete adapter/backend is selected.",
                planned_tools: &[
                    "Bus Scan",
                    "Register Read/Write",
                    "EEPROM Tool",
                    "Raw transaction console",
                ],
            },
        ),
        Screen::Spi => render_placeholder_protocol(
            frame,
            PlaceholderProtocolView {
                protocol: "SPI",
                title: " SPI PROTOCOL WORKBENCH ",
                color: Color::LightBlue,
                summary: " SPI tools will live here once the concrete adapter/backend is selected.",
                planned_tools: &[
                    "Raw Transfer",
                    "SPI Flash Tool",
                    "Device command console",
                    "Preset-based transaction runner",
                ],
            },
        ),
        Screen::SerialMonitor => monitor::render(frame, &app.serial_monitor),
        Screen::Flasher => flasher::render(frame, &app.flasher),
    }
}

struct ProbeProtocolView {
    protocol: &'static str,
    title: &'static str,
    badge: &'static str,
    color: Color,
    summary: &'static str,
    details: &'static [&'static str],
}

struct PlaceholderProtocolView {
    protocol: &'static str,
    title: &'static str,
    color: Color,
    summary: &'static str,
    planned_tools: &'static [&'static str],
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

fn render_probe_protocol(frame: &mut Frame, view: ProbeProtocolView) {
    let area = frame.area();
    let [header, body, notes, hint] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Fill(1),
        Constraint::Length(5),
        Constraint::Length(1),
    ])
    .areas(area);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(view.title, theme::selected_style()),
                Span::styled(" Debug probe lane active ", theme::title_style()),
            ]),
            Line::from(view.summary),
        ])
        .alignment(Alignment::Center)
        .block(theme::active_panel_block(" Debug Assistant ")),
        header,
    );

    let [nav, detail] =
        Layout::horizontal([Constraint::Length(28), Constraint::Fill(1)]).areas(body);
    render_probe_tool_select(frame, view.protocol, view.color, nav);
    render_probe_tool_detail(frame, view, detail);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  PROTOCOL NOTES  ", theme::title_style()),
                Span::styled("probe-rs backend", theme::muted_style()),
            ]),
            Line::from("  Flashing uses probe-rs for probe access and target attachment."),
            Line::from("  Additional inspect/debug tools can be added to this lane later."),
        ])
        .block(
            Block::new()
                .title(" Lane Notes ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::MUTED)),
        ),
        notes,
    );

    frame.render_widget(
        Paragraph::new("  Enter:Open Flash  Esc:Protocol dashboard  q:Quit")
            .style(theme::muted_style()),
        hint,
    );
}

fn render_probe_tool_select(frame: &mut Frame, protocol: &str, color: Color, area: Rect) {
    let block = theme::panel_block(" Tool Select ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([Constraint::Length(4), Constraint::Fill(1)]).split(inner);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("  > Flash"),
            Line::from(format!("    {} probe", protocol)),
            Line::from("    Firmware download"),
        ])
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ),
        rows[0],
    );
}

fn render_probe_tool_detail(frame: &mut Frame, view: ProbeProtocolView, area: Rect) {
    let block = Block::new()
        .title(format!(" Selected Tool — {} Flash ", view.protocol))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(view.color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut detail = Vec::with_capacity(view.details.len() + 4);
    detail.push(Line::from(Span::styled(
        view.badge,
        Style::default().fg(view.color).add_modifier(Modifier::BOLD),
    )));
    detail.push(Line::from(""));
    for line in view.details {
        detail.push(Line::from(format!("  - {}", line)));
    }
    detail.push(Line::from(""));
    detail.push(Line::from(Span::styled(
        "  Press Enter to open this protocol tool.",
        theme::muted_style(),
    )));

    frame.render_widget(Paragraph::new(detail), inner);
}

fn render_placeholder_protocol(frame: &mut Frame, view: PlaceholderProtocolView) {
    let area = frame.area();
    let [header, body, notes, hint] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Fill(1),
        Constraint::Length(5),
        Constraint::Length(1),
    ])
    .areas(area);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(view.title, theme::selected_style()),
                Span::styled(" planned protocol lane ", theme::title_style()),
            ]),
            Line::from(view.summary),
        ])
        .alignment(Alignment::Center)
        .block(theme::active_panel_block(" Debug Assistant ")),
        header,
    );

    let block = Block::new()
        .title(format!(" {} Tool Plan ", view.protocol))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(view.color));
    let inner = block.inner(body);
    frame.render_widget(block, body);

    let mut lines = Vec::with_capacity(view.planned_tools.len() + 5);
    lines.push(Line::from(Span::styled(
        "  BACKEND PENDING",
        Style::default().fg(view.color).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    for tool in view.planned_tools {
        lines.push(Line::from(format!("  - {}", tool)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  No hardware transport is initialized yet; this is a protocol lane skeleton.",
        theme::muted_style(),
    )));
    frame.render_widget(Paragraph::new(lines), inner);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  PROTOCOL NOTES  ", theme::title_style()),
                Span::styled("implementation deferred", theme::muted_style()),
            ]),
            Line::from(
                "  Add transport/i2c.rs or transport/spi.rs only after choosing an adapter.",
            ),
            Line::from("  Add protocol.rs only for real device-level command implementations."),
        ])
        .block(
            Block::new()
                .title(" Lane Notes ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::MUTED)),
        ),
        notes,
    );

    frame.render_widget(
        Paragraph::new("  Esc:Protocol dashboard  q:Quit").style(theme::muted_style()),
        hint,
    );
}
