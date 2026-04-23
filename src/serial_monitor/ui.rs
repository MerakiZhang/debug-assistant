use super::state::{ConfigField, Direction, DisplayMode, Focus, LogEntry, SerialMonitorState};
use crate::serial::{
    data_bits_label, flow_control_label, parity_label, parity_short, stop_bits_label, BAUD_PRESETS,
    DATA_BITS_OPTIONS, FLOW_CONTROL_OPTIONS, PARITY_OPTIONS, STOP_BITS_OPTIONS,
};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn render(frame: &mut Frame, state: &SerialMonitorState) {
    let [rx_area, tx_area, status_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_receive_panel(frame, state, rx_area);
    render_send_panel(frame, state, tx_area);
    render_status_bar(frame, state, status_area);

    if state.show_config {
        let popup = centered_rect(62, 76, frame.area());
        frame.render_widget(Clear, popup);
        render_config_popup(frame, state, popup);
    }
    if state.show_help {
        let popup = centered_rect(52, 88, frame.area());
        frame.render_widget(Clear, popup);
        render_help_overlay(frame, popup);
    }
}

fn render_receive_panel(frame: &mut Frame, state: &SerialMonitorState, area: Rect) {
    let focused = state.focus == Focus::Receive && !state.show_config && !state.show_help;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let auto_str = if state.auto_scroll {
        "Auto↓:ON "
    } else {
        "Auto↓:OFF"
    };
    let title = format!(
        " Receive ─ [{}] [{}] [F3:Clear] ",
        state.display_mode.label(),
        auto_str
    );

    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    state.rx_visible_rows.set(inner.height);
    let visible = inner.height as usize;

    let lines: Vec<Line<'static>> = state
        .log
        .iter()
        .flat_map(|e| format_log_entry(e, state.display_mode))
        .collect();

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
            Span::styled(format!("[{}] ", ts), Style::default().fg(Color::DarkGray)),
            Span::styled(
                text,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])];
    }

    let (dir_label, dir_color) = match entry.direction {
        Direction::Rx => ("RX", Color::Green),
        Direction::Tx => ("TX", Color::Yellow),
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

fn render_send_panel(frame: &mut Frame, state: &SerialMonitorState, area: Rect) {
    let focused = state.focus == Focus::Send && !state.show_config && !state.show_help;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let hex_str = if state.hex_send_mode {
        "HEX:ON "
    } else {
        "HEX:off"
    };
    let hist_hint = if !state.send_history.is_empty() {
        format!("[History:{}] ", state.send_history.len())
    } else {
        String::new()
    };
    let title = format!(
        " Send ─ [Suffix:{}] [{}] {}[Ctrl+H:hex] [Ctrl+N:suffix] ",
        state.newline_suffix.label(),
        hex_str,
        hist_hint
    );

    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);

    let display = format!("> {}", state.input_buf);
    let para = Paragraph::new(display.as_str()).block(block);
    frame.render_widget(para, area);

    if focused {
        let display_col = UnicodeWidthStr::width(&state.input_buf[..state.cursor_pos]) as u16;
        let cursor_x = (inner.x + 2 + display_col).min(inner.x + inner.width.saturating_sub(1));
        frame.set_cursor_position((cursor_x, inner.y));
    }
}

fn render_status_bar(frame: &mut Frame, state: &SerialMonitorState, area: Rect) {
    let (symbol, sym_color, label) = if state.connected {
        ("●", Color::Green, "CONNECTED  ")
    } else {
        ("○", Color::Red, "DISCONNECTED")
    };

    let config_str = if !state.serial_config.port_name.is_empty()
        && state.serial_config.port_name != "(no ports found)"
    {
        format!(
            "  {} {} {}{}{}",
            state.serial_config.port_name,
            state.serial_config.baud_rate,
            data_bits_label(state.serial_config.data_bits),
            parity_short(state.serial_config.parity),
            stop_bits_label(state.serial_config.stop_bits),
        )
    } else {
        "  (no port — press F2)".to_string()
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" {}", symbol),
            Style::default().fg(sym_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(label, Style::default().fg(sym_color)),
        Span::raw(config_str),
        Span::raw(format!("  TX:{}", fmt_bytes(state.bytes_tx))),
        Span::raw(format!("  RX:{}", fmt_bytes(state.bytes_rx))),
        Span::styled(
            "  │  F1:Help F2:Config F3:Clear F4:Mode F5:AutoScroll Tab:Focus Esc:Home q:Quit",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_config_popup(frame: &mut Frame, state: &SerialMonitorState, area: Rect) {
    let block = Block::new()
        .title(" Serial Port Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let port_val = if state.config_port_list.is_empty() {
        "(none)".to_string()
    } else {
        state.config_port_list[state.config_port_idx].clone()
    };

    let field_rows: &[(ConfigField, &str, String)] = &[
        (ConfigField::PortName, "Port Name   ", port_val),
        (
            ConfigField::BaudRate,
            "Baud Rate   ",
            BAUD_PRESETS[state.config_baud_idx].to_string(),
        ),
        (
            ConfigField::DataBits,
            "Data Bits   ",
            data_bits_label(DATA_BITS_OPTIONS[state.config_databits_idx]).to_string(),
        ),
        (
            ConfigField::StopBits,
            "Stop Bits   ",
            stop_bits_label(STOP_BITS_OPTIONS[state.config_stopbits_idx]).to_string(),
        ),
        (
            ConfigField::Parity,
            "Parity      ",
            parity_label(PARITY_OPTIONS[state.config_parity_idx]).to_string(),
        ),
        (
            ConfigField::FlowControl,
            "Flow Control",
            flow_control_label(FLOW_CONTROL_OPTIONS[state.config_flow_idx]).to_string(),
        ),
    ];

    for (i, (field, label, value)) in field_rows.iter().enumerate() {
        let focused = state.config_field == *field;
        let (row_style, arrow) = if focused {
            (
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                "◄ ► ",
            )
        } else {
            (Style::default(), "    ")
        };
        let text = format!("  {}: {}{}", label, arrow, value);
        frame.render_widget(Paragraph::new(text).style(row_style), rows[i]);
    }

    let btn = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            " Enter:Connect ",
            Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            " Esc:Cancel ",
            Style::default().bg(Color::Red).fg(Color::White),
        ),
        Span::styled(
            "   ↑↓/Tab:Navigate  ←→:Change",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(btn), rows[7]);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .title(" Keyboard Shortcuts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let h = || {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };
    let d = || Style::default().fg(Color::DarkGray);

    let lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled("  Global", h())),
        Line::from("  F1              Open help (any key closes)"),
        Line::from("  F2              Open config / connect"),
        Line::from("  F3              Clear receive log"),
        Line::from("  F4              Cycle display mode (ASCII → HEX → BOTH)"),
        Line::from("  F5              Toggle auto-scroll"),
        Line::from("  Tab             Switch focus: Receive ↔ Send"),
        Line::from("  Ctrl+D          Disconnect"),
        Line::from("  Esc             Return to home menu"),
        Line::from("  Ctrl+C / q      Quit"),
        Line::from(""),
        Line::from(Span::styled("  Send Panel", h())),
        Line::from("  Enter           Send current input"),
        Line::from("  ↑ / ↓           Browse send history"),
        Line::from("  ← / →           Move cursor left/right"),
        Line::from("  Home / End       Jump to start / end"),
        Line::from("  Backspace/Del    Delete character"),
        Line::from("  Ctrl+H          Toggle HEX send mode"),
        Line::from("  Ctrl+N          Cycle newline suffix (None→CR→LF→CRLF)"),
        Line::from(""),
        Line::from(Span::styled("  Receive Panel", h())),
        Line::from("  ↑ / ↓           Scroll one line"),
        Line::from("  PgUp / PgDn     Scroll one page"),
        Line::from("  Home / End       Jump to top / bottom"),
        Line::from(""),
        Line::from(Span::styled("  Config Popup", h())),
        Line::from("  ↑ / ↓ / Tab     Next/prev field"),
        Line::from("  Shift+Tab       Previous field"),
        Line::from("  ← / →           Change value"),
        Line::from("  Enter           Apply settings and connect"),
        Line::from("  Esc             Cancel"),
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
