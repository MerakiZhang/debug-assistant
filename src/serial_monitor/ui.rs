use super::state::{ConfigField, Direction, DisplayMode, Focus, LogEntry, SerialMonitorState};
use crate::serial::{
    data_bits_label, flow_control_label, parity_label, parity_short, stop_bits_label, BAUD_PRESETS,
    DATA_BITS_OPTIONS, FLOW_CONTROL_OPTIONS, PARITY_OPTIONS, STOP_BITS_OPTIONS,
};
use crate::ui_theme as theme;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn render(frame: &mut Frame, state: &SerialMonitorState) {
    let area = frame.area();
    if area.width >= 90 {
        let [workspace, tx_area, shortcuts_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .areas(area);
        let [setup_area, traffic_area] =
            Layout::horizontal([Constraint::Length(28), Constraint::Fill(1)]).areas(workspace);

        render_setup_panel(frame, state, setup_area);
        render_traffic_panel(frame, state, traffic_area);
        render_send_line(frame, state, tx_area);
        render_shortcuts_bar(frame, shortcuts_area);
    } else {
        let [setup_area, traffic_area, tx_area, shortcuts_area] = Layout::vertical([
            Constraint::Length(10),
            Constraint::Fill(1),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .areas(area);

        render_setup_panel(frame, state, setup_area);
        render_traffic_panel(frame, state, traffic_area);
        render_send_line(frame, state, tx_area);
        render_shortcuts_bar(frame, shortcuts_area);
    }

    if state.show_help {
        let popup = centered_rect(52, 88, frame.area());
        frame.render_widget(Clear, popup);
        render_help_overlay(frame, popup);
    }
}

fn render_setup_panel(frame: &mut Frame, state: &SerialMonitorState, area: Rect) {
    let focused = state.focus == Focus::Setup;
    let border_color = if focused { theme::ACCENT } else { theme::MUTED };

    let title = if focused {
        " Serial Setup "
    } else {
        " Serial Link "
    };

    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if focused {
        render_setup_fields(frame, state, inner);
    } else {
        render_link_status(frame, state, inner);
    }
}

fn render_link_status(frame: &mut Frame, state: &SerialMonitorState, area: Rect) {
    let (symbol, sym_color, label) = if state.connected {
        ("●", theme::SUCCESS, "CONNECTED")
    } else {
        ("○", theme::DANGER, "DISCONNECTED")
    };

    let port = if !state.serial_config.port_name.is_empty()
        && state.serial_config.port_name != "(no ports found)"
    {
        state.serial_config.port_name.as_str()
    } else {
        "(no port)"
    };
    let frame_cfg = format!(
        "{} {}{}{}",
        state.serial_config.baud_rate,
        data_bits_label(state.serial_config.data_bits),
        parity_short(state.serial_config.parity),
        stop_bits_label(state.serial_config.stop_bits),
    );
    let send_mode = if state.hex_send_mode { "HEX" } else { "TEXT" };
    let auto = if state.auto_scroll { "ON" } else { "OFF" };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", symbol),
                Style::default().fg(sym_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(label, Style::default().fg(sym_color)),
        ]),
        Line::from(format!("  Port      {}", port)),
        Line::from(format!("  Frame     {}", frame_cfg)),
        Line::from(format!("  View      {}", state.display_mode.label())),
        Line::from(format!(
            "  Send      {} + {}",
            send_mode,
            state.newline_suffix.label()
        )),
        Line::from(format!("  AutoScroll {}", auto)),
        Line::from(format!(
            "  RX {}   TX {}",
            fmt_bytes(state.bytes_rx),
            fmt_bytes(state.bytes_tx)
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Tab / F2: Setup",
            theme::muted_style(),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_setup_fields(frame: &mut Frame, state: &SerialMonitorState, area: Rect) {
    let port_val = if state.config_port_list.is_empty() {
        "(none)".to_string()
    } else {
        state.config_port_list[state.config_port_idx].clone()
    };

    let (conn_sym, conn_color, conn_label) = if state.connected {
        ("●", theme::SUCCESS, "CONNECTED")
    } else {
        ("○", theme::DANGER, "DISCONNECTED")
    };

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", conn_sym),
                Style::default()
                    .fg(conn_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(conn_label, Style::default().fg(conn_color)),
        ]),
        Line::from(""),
    ];

    let fields: &[(ConfigField, &str, String)] = &[
        (ConfigField::PortName, "Port ", port_val),
        (
            ConfigField::BaudRate,
            "Baud ",
            BAUD_PRESETS[state.config_baud_idx].to_string(),
        ),
        (
            ConfigField::DataBits,
            "Data ",
            data_bits_label(DATA_BITS_OPTIONS[state.config_databits_idx]).to_string(),
        ),
        (
            ConfigField::StopBits,
            "Stop ",
            stop_bits_label(STOP_BITS_OPTIONS[state.config_stopbits_idx]).to_string(),
        ),
        (
            ConfigField::Parity,
            "Parity",
            parity_label(PARITY_OPTIONS[state.config_parity_idx]).to_string(),
        ),
        (
            ConfigField::FlowControl,
            "Flow  ",
            flow_control_label(FLOW_CONTROL_OPTIONS[state.config_flow_idx]).to_string(),
        ),
    ];

    for (field, label, value) in fields {
        let sel = state.config_field == *field;
        let style = if sel {
            theme::selected_style()
        } else {
            Style::default()
        };
        let arrows = if sel { "◄ ► " } else { "    " };
        let hint = if sel && *field == ConfigField::PortName {
            " R:refresh"
        } else {
            ""
        };
        lines.push(Line::from(vec![
            Span::styled(if sel { ">" } else { " " }, style),
            Span::styled(format!(" {} ", label), theme::title_style()),
            Span::styled(arrows.to_string(), style),
            Span::styled(value.clone(), style),
            Span::styled(hint, theme::muted_style()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Enter", theme::selected_style()),
        Span::styled(":Connect  ", theme::muted_style()),
        Span::styled("Esc", Style::default().fg(theme::DANGER)),
        Span::styled(":Cancel", theme::muted_style()),
    ]));

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_traffic_panel(frame: &mut Frame, state: &SerialMonitorState, area: Rect) {
    let focused = state.focus == Focus::Receive;
    let border_style = if focused {
        Style::default().fg(theme::ACCENT)
    } else {
        theme::muted_style()
    };
    let title = format!(" Traffic Monitor ─ {} ", state.display_mode.label());

    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    state.rx_visible_rows.set(inner.height);
    let visible = inner.height as usize;

    let lines: Vec<Line<'static>> = if state.log.is_empty() {
        vec![Line::from(Span::styled(
            "  No traffic yet. Press Tab or F2 to configure.",
            theme::muted_style(),
        ))]
    } else {
        state
            .log
            .iter()
            .flat_map(|e| format_log_entry(e, state.display_mode))
            .collect()
    };

    let total = lines.len();
    let scroll = state.log_scroll.min(total.saturating_sub(visible)) as u16;

    let para = Paragraph::new(lines).block(block).scroll((scroll, 0));
    frame.render_widget(para, area);

    if total > visible {
        let mut sb = ScrollbarState::new(total.saturating_sub(visible)).position(scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut sb,
        );
    }
}

fn format_log_entry(entry: &LogEntry, mode: DisplayMode) -> Vec<Line<'static>> {
    let ts = entry.timestamp.format("%H:%M:%S%.3f").to_string();

    if entry.direction == Direction::Status {
        let text = String::from_utf8_lossy(&entry.raw).into_owned();
        return vec![Line::from(vec![
            Span::styled(format!("[{}] ", ts), theme::muted_style()),
            Span::styled(
                text,
                Style::default()
                    .fg(theme::MUTED)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])];
    }

    let (dir_label, dir_color) = match entry.direction {
        Direction::Rx => ("RX", theme::SUCCESS),
        Direction::Tx => ("TX", theme::WARNING),
        Direction::Status => unreachable!(),
    };

    let prefix = Span::styled(
        format!("[{} {}] ", ts, dir_label),
        Style::default().fg(dir_color).add_modifier(Modifier::BOLD),
    );

    let content: String = match mode {
        DisplayMode::Ascii => decode_utf8_display(&entry.raw),
        DisplayMode::Hex => entry.raw.iter().map(|b| format!("{:02X} ", b)).collect(),
        DisplayMode::Both => {
            let hex: String = entry.raw.iter().map(|b| format!("{:02X} ", b)).collect();
            let ascii: String = entry
                .raw
                .iter()
                .map(|&b| {
                    if b.is_ascii_graphic() || b == b' ' {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            format!("{}  {}", hex.trim_end(), ascii)
        }
    };

    vec![Line::from(vec![
        prefix,
        Span::styled(content, Style::default().fg(dir_color)),
    ])]
}

fn render_send_line(frame: &mut Frame, state: &SerialMonitorState, area: Rect) {
    let focused = state.focus == Focus::Send;
    let border_style = if focused {
        Style::default().fg(theme::ACCENT)
    } else {
        theme::muted_style()
    };
    let hist_hint = if !state.send_history.is_empty() {
        format!("History:{}", state.send_history.len())
    } else {
        "History:0".to_string()
    };
    let title = format!(
        " Send Line ─ {} ─ {} ",
        hist_hint,
        state.newline_suffix.label()
    );

    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);

    let prompt = if state.hex_send_mode {
        "HEX  > "
    } else {
        "TEXT > "
    };
    let display = format!(" {}{}", prompt, state.input_buf);
    let para = Paragraph::new(display.as_str()).block(block);
    frame.render_widget(para, area);

    if focused {
        let prefix_width = UnicodeWidthStr::width(format!(" {}", prompt).as_str()) as u16;
        let input_width = UnicodeWidthStr::width(&state.input_buf[..state.cursor_pos]) as u16;
        let cursor_x =
            (inner.x + prefix_width + input_width).min(inner.x + inner.width.saturating_sub(1));
        frame.set_cursor_position((cursor_x, inner.y));
    }
}

fn render_shortcuts_bar(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(
            "  F1:Help F2:Setup F3:Clear F4:Mode F5:Auto F6:Copy F7:Save Tab:Focus Ctrl+D:Disc Esc:Back",
        )
        .style(theme::muted_style()),
        area,
    );
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let block = theme::status_panel_block(" Keyboard Shortcuts ", theme::WARNING);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let h = || {
        Style::default()
            .fg(theme::WARNING)
            .add_modifier(Modifier::BOLD)
    };
    let d = || theme::muted_style();

    let lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled("  Global", h())),
        Line::from("  F1              Open help (any key closes)"),
        Line::from("  F2              Focus Setup panel"),
        Line::from("  F3              Clear receive log"),
        Line::from("  F4              Cycle display mode (ASCII → HEX → BOTH)"),
        Line::from("  F5              Toggle auto-scroll"),
        Line::from("  F6              Copy log to clipboard"),
        Line::from("  F7              Save log to logs/"),
        Line::from("  Tab             Cycle focus: Setup → Receive → Send"),
        Line::from("  Shift+Tab       Reverse cycle"),
        Line::from("  Esc             Back to Serial page (when not in Setup)"),
        Line::from("  Ctrl+C          Quit"),
        Line::from(""),
        Line::from(Span::styled("  Setup Panel (Tab / F2 to enter)", h())),
        Line::from("  ↑ / ↓           Navigate fields"),
        Line::from("  ← / →           Change value"),
        Line::from("  R               Refresh port list (on Port field)"),
        Line::from("  Enter           Apply settings and connect"),
        Line::from("  Ctrl+D          Disconnect only"),
        Line::from("  Esc             Discard changes, focus Send"),
        Line::from(""),
        Line::from(Span::styled("  Send Panel", h())),
        Line::from("  Enter           Send current input"),
        Line::from("  ↑ / ↓           Browse send history"),
        Line::from("  ← / →           Move cursor left/right"),
        Line::from("  Home / End       Jump to start / end"),
        Line::from("  Backspace/Del    Delete character"),
        Line::from("  Ctrl+H          Toggle HEX send mode"),
        Line::from("  Ctrl+N          Cycle newline suffix (None→CR→LF→CRLF)"),
        Line::from("  Ctrl+D          Disconnect"),
        Line::from(""),
        Line::from(Span::styled("  Receive Panel", h())),
        Line::from("  ↑ / ↓           Scroll one line"),
        Line::from("  PgUp / PgDn     Scroll one page"),
        Line::from("  Home / End       Jump to top / bottom"),
        Line::from(""),
        Line::from(Span::styled("  (press any key to close)", d())),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area)[1];
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(v)[1]
}

fn decode_utf8_display(raw: &[u8]) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        match std::str::from_utf8(&raw[i..]) {
            Ok(s) => {
                for c in s.chars() {
                    match c {
                        '\r' | '\n' => {}
                        '\t' => out.push_str("  "),
                        c if c.is_control() => out.push_str(&format!("[{:02X}]", c as u32)),
                        c => out.push(c),
                    }
                }
                break;
            }
            Err(e) => {
                let valid = std::str::from_utf8(&raw[i..i + e.valid_up_to()]).unwrap();
                for c in valid.chars() {
                    match c {
                        '\r' | '\n' => {}
                        '\t' => out.push_str("  "),
                        c if c.is_control() => out.push_str(&format!("[{:02X}]", c as u32)),
                        c => out.push(c),
                    }
                }
                i += e.valid_up_to();
                let bad = e.error_len().unwrap_or(1);
                for &b in &raw[i..i + bad] {
                    out.push_str(&format!("\\x{:02X}", b));
                }
                i += bad;
            }
        }
    }
    out
}

fn fmt_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{}B", n)
    } else if n < 1024 * 1024 {
        format!("{:.1}K", n as f64 / 1024.0)
    } else {
        format!("{:.1}M", n as f64 / (1024.0 * 1024.0))
    }
}
