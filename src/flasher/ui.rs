use super::state::{
    FlasherMethod, FlasherState, FlasherSubScreen, IspBootMode, IspConfigField, JtagConfigField,
    SwdConfigField,
};
use crate::serial::ISP_BAUD_PRESETS;
use crate::ui_theme as theme;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Gauge, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn render(frame: &mut Frame, state: &FlasherState) {
    match state.method {
        FlasherMethod::UsartIsp => render_isp_workbench(frame, state),
        FlasherMethod::Jtag => render_jtag_workbench(frame, state),
        FlasherMethod::Swd => render_swd_workbench(frame, state),
    }
}

fn render_isp_workbench(frame: &mut Frame, state: &FlasherState) {
    let area = frame.area();
    let [title_area, body_area, file_area, gauge_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(area);

    render_workbench_title(frame, title_area, state, "Serial ISP Workbench");

    let [setup_area, log_area] =
        Layout::horizontal([Constraint::Length(32), Constraint::Fill(1)]).areas(body_area);
    render_isp_workbench_config(frame, state, setup_area);
    render_isp_file_input(frame, state, file_area);
    render_operation_log(frame, state, log_area, " Operation Log ");

    render_progress_gauge(frame, state, gauge_area);

    let hint = match state.sub_screen {
        FlasherSubScreen::Config => {
            "  ↑↓/Tab:Field  ←→:Change/File cursor  Home/End:File cursor  F6:Copy F7:Save  Enter:Start  Esc:Back"
        }
        FlasherSubScreen::Progress if state.op_done => {
            "  Up/Down/PgUp/PgDn/Home/End: Scroll log  F6:Copy F7:Save  Esc: Back to config"
        }
        FlasherSubScreen::Progress if state.cancel_armed => {
            "  Up/Down/PgUp/PgDn/Home/End: Scroll log  F6:Copy F7:Save  Esc: Confirm cancel"
        }
        FlasherSubScreen::Progress => {
            "  Up/Down/PgUp/PgDn/Home/End: Scroll log  F6:Copy F7:Save  Esc: Arm cancel"
        }
    };
    frame.render_widget(Paragraph::new(hint).style(theme::muted_style()), hint_area);
}

fn render_jtag_workbench(frame: &mut Frame, state: &FlasherState) {
    let area = frame.area();
    let [title_area, body_area, file_area, gauge_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(area);

    render_workbench_title(frame, title_area, state, "JTAG Workbench");
    let [setup_area, log_area] =
        Layout::horizontal([Constraint::Length(34), Constraint::Fill(1)]).areas(body_area);
    render_jtag_setup_panel(frame, state, setup_area);
    render_firmware_file_input(
        frame,
        state,
        file_area,
        &state.jtag_file_path,
        state.jtag_file_cursor,
        state.sub_screen == FlasherSubScreen::Config
            && state.jtag_field == JtagConfigField::FilePath,
    );
    render_operation_log(frame, state, log_area, " Operation Log ");
    render_progress_gauge(frame, state, gauge_area);

    let hint = match state.sub_screen {
        FlasherSubScreen::Config => {
            "  ↑↓/Tab:Field  ←→:Change  Type:Base/Chip/File  R:Refresh  F6:Copy F7:Save  Enter:Start  Esc:Back"
        }
        FlasherSubScreen::Progress if state.op_done => {
            "  Up/Down/PgUp/PgDn/Home/End: Scroll log  F6:Copy F7:Save  Esc: Back to config"
        }
        FlasherSubScreen::Progress if state.cancel_armed => {
            "  Up/Down/PgUp/PgDn/Home/End: Scroll log  F6:Copy F7:Save  Esc: Confirm cancel"
        }
        FlasherSubScreen::Progress => {
            "  Up/Down/PgUp/PgDn/Home/End: Scroll log  F6:Copy F7:Save  Esc: Arm cancel"
        }
    };
    frame.render_widget(Paragraph::new(hint).style(theme::muted_style()), hint_area);
}

fn render_swd_workbench(frame: &mut Frame, state: &FlasherState) {
    let area = frame.area();
    let [title_area, body_area, file_area, gauge_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(area);

    render_workbench_title(frame, title_area, state, "SWD Workbench");
    let [setup_area, log_area] =
        Layout::horizontal([Constraint::Length(38), Constraint::Fill(1)]).areas(body_area);
    render_swd_setup_panel(frame, state, setup_area);
    render_firmware_file_input(
        frame,
        state,
        file_area,
        &state.swd_file_path,
        state.swd_file_cursor,
        state.sub_screen == FlasherSubScreen::Config && state.swd_field == SwdConfigField::FilePath,
    );
    render_operation_log(frame, state, log_area, " Operation Log ");
    render_progress_gauge(frame, state, gauge_area);

    let hint = match state.sub_screen {
        FlasherSubScreen::Config => {
            "  ↑↓/Tab:Field  ←→:Change  Type:Addr/Chip/File  R:Refresh  F6:Copy F7:Save  Enter:Start  Esc:Back"
        }
        FlasherSubScreen::Progress if state.op_done => {
            "  Up/Down/PgUp/PgDn/Home/End: Scroll log  F6:Copy F7:Save  Esc: Back to config"
        }
        FlasherSubScreen::Progress if state.cancel_armed => {
            "  Up/Down/PgUp/PgDn/Home/End: Scroll log  F6:Copy F7:Save  Esc: Confirm cancel"
        }
        FlasherSubScreen::Progress => {
            "  Up/Down/PgUp/PgDn/Home/End: Scroll log  F6:Copy F7:Save  Esc: Arm cancel"
        }
    };
    frame.render_widget(Paragraph::new(hint).style(theme::muted_style()), hint_area);
}

fn render_workbench_title(frame: &mut Frame, area: Rect, state: &FlasherState, title: &str) {
    let title_color = if state.sub_screen == FlasherSubScreen::Progress {
        if state.op_done {
            if state.op_ok {
                theme::SUCCESS
            } else {
                theme::DANGER
            }
        } else {
            theme::WARNING
        }
    } else {
        theme::ACCENT
    };
    let status = match state.sub_screen {
        FlasherSubScreen::Config => "Ready",
        FlasherSubScreen::Progress if state.op_done && state.op_ok => "Complete",
        FlasherSubScreen::Progress if state.op_done => "Failed",
        FlasherSubScreen::Progress => "Running",
    };
    frame.render_widget(
        Paragraph::new(format!(" {} | STATUS: {} ", title, status))
            .style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(title_color)),
            ),
        area,
    );
}

fn render_isp_workbench_config(frame: &mut Frame, state: &FlasherState, area: Rect) {
    let active = state.sub_screen == FlasherSubScreen::Config;
    let config_block = Block::new()
        .title(if active {
            " Setup / Boot "
        } else {
            " Setup / Boot (Locked) "
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if active { theme::ACCENT } else { theme::MUTED }));
    let config_inner = config_block.inner(area);
    frame.render_widget(config_block, area);

    let port_val = if state.isp_port_list.is_empty() {
        "(none)".to_string()
    } else {
        state.isp_port_list[state.isp_port_idx].clone()
    };

    let port_style = isp_field_style(state, active, IspConfigField::Port);
    let baud_style = isp_field_style(state, active, IspConfigField::BaudRate);
    let boot_style = isp_field_style(state, active, IspConfigField::BootMode);
    let note_lines = if state.isp_boot_mode == IspBootMode::Manual {
        [
            ("  Set BOOT0 high", Style::default().fg(theme::WARNING)),
            (
                "  Reset target before flashing",
                Style::default().fg(theme::WARNING),
            ),
            ("  File input stays editable", theme::muted_style()),
            ("  until Enter starts download.", theme::muted_style()),
        ]
    } else {
        [
            (
                "  RTS drives BOOT0 high",
                Style::default().fg(theme::WARNING),
            ),
            (
                "  DTR pulses RESET low",
                Style::default().fg(theme::WARNING),
            ),
            ("  Check RTS->BOOT0 and", theme::muted_style()),
            ("  DTR->RESET wiring on fail.", theme::muted_style()),
        ]
    };

    let setup = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Port", theme::title_style()),
            Span::raw("  "),
            Span::styled(
                if state.isp_field == IspConfigField::Port {
                    "◄ ► "
                } else {
                    ""
                },
                port_style,
            ),
            Span::styled(port_val, port_style),
        ]),
        Line::from(vec![
            Span::styled("Baud", theme::title_style()),
            Span::raw("  "),
            Span::styled(
                if state.isp_field == IspConfigField::BaudRate {
                    "◄ ► "
                } else {
                    ""
                },
                baud_style,
            ),
            Span::styled(ISP_BAUD_PRESETS[state.isp_baud_idx].to_string(), baud_style),
        ]),
        Line::from(vec![
            Span::styled("Boot", theme::title_style()),
            Span::raw("  "),
            Span::styled(
                if state.isp_field == IspConfigField::BootMode {
                    "◄ ► "
                } else {
                    ""
                },
                boot_style,
            ),
            Span::styled(state.isp_boot_mode.label(), boot_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("Notes", theme::title_style())),
        Line::from(Span::styled(note_lines[0].0, note_lines[0].1)),
        Line::from(Span::styled(note_lines[1].0, note_lines[1].1)),
        Line::from(Span::styled(note_lines[2].0, note_lines[2].1)),
        Line::from(Span::styled(note_lines[3].0, note_lines[3].1)),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(setup, config_inner);
}

fn render_jtag_setup_panel(frame: &mut Frame, state: &FlasherState, area: Rect) {
    let active = state.sub_screen == FlasherSubScreen::Config;
    let block = Block::new()
        .title(if active {
            " Setup "
        } else {
            " Setup (Locked) "
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if active { theme::ACCENT } else { theme::MUTED }));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let probe_val = if state.jtag_probe_list.is_empty() {
        "(no probes found)".to_string()
    } else {
        state.jtag_probe_list[state.jtag_probe_idx].clone()
    };
    let verify = if state.jtag_verify {
        "Enabled"
    } else {
        "Disabled"
    };
    let reset = if state.jtag_reset_run {
        "Enabled"
    } else {
        "Disabled"
    };
    let chip = if state.jtag_chip_name.is_empty() {
        "(required)"
    } else {
        state.jtag_chip_name.as_str()
    };
    let file_state = if state.jtag_file_path.is_empty() {
        "No file selected"
    } else {
        "File ready"
    };

    let panel = Paragraph::new(vec![
        setup_cycle_line(
            "Probe",
            &probe_val,
            state.jtag_field == JtagConfigField::Probe,
            jtag_field_style(state, active, JtagConfigField::Probe),
        ),
        setup_cycle_line(
            "Speed",
            &format!("{} kHz", state.jtag_speed_khz()),
            state.jtag_field == JtagConfigField::Speed,
            jtag_field_style(state, active, JtagConfigField::Speed),
        ),
        setup_cycle_line(
            "Verify",
            verify,
            state.jtag_field == JtagConfigField::Verify,
            jtag_field_style(state, active, JtagConfigField::Verify),
        ),
        setup_cycle_line(
            "Reset",
            reset,
            state.jtag_field == JtagConfigField::ResetRun,
            jtag_field_style(state, active, JtagConfigField::ResetRun),
        ),
        setup_value_line(
            "Base",
            &state.jtag_bin_base_address,
            state.jtag_field == JtagConfigField::BinBaseAddress,
            jtag_field_style(state, active, JtagConfigField::BinBaseAddress),
        ),
        setup_cycle_line(
            "Preset",
            state.jtag_chip_preset(),
            state.jtag_field == JtagConfigField::ChipPreset,
            jtag_field_style(state, active, JtagConfigField::ChipPreset),
        ),
        setup_value_line(
            "Chip",
            chip,
            state.jtag_field == JtagConfigField::ChipName,
            jtag_field_style(state, active, JtagConfigField::ChipName),
        ),
        setup_value_line(
            "File",
            file_state,
            state.jtag_field == JtagConfigField::FilePath,
            jtag_field_style(state, active, JtagConfigField::FilePath),
        ),
        Line::from(""),
        Line::from(Span::styled("Notes", theme::title_style())),
        Line::from(Span::styled(
            "  Select options with ← / →",
            Style::default().fg(theme::WARNING),
        )),
        Line::from(Span::styled(
            "  BIN files use Base address",
            theme::muted_style(),
        )),
        Line::from(Span::styled(
            "  Press R on Probe to refresh",
            theme::muted_style(),
        )),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(panel, inner);
}

fn render_swd_setup_panel(frame: &mut Frame, state: &FlasherState, area: Rect) {
    let active = state.sub_screen == FlasherSubScreen::Config;
    let block = Block::new()
        .title(if active {
            " Setup "
        } else {
            " Setup (Locked) "
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if active { theme::ACCENT } else { theme::MUTED }));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let probe_val = if state.swd_probe_list.is_empty() {
        "(no probes found)".to_string()
    } else {
        state.swd_probe_list[state.swd_probe_idx].clone()
    };
    let verify = if state.swd_verify {
        "Enabled"
    } else {
        "Disabled"
    };
    let reset = if state.swd_reset_run {
        "Enabled"
    } else {
        "Disabled"
    };
    let chip = if state.swd_chip_name.is_empty() {
        "(required)"
    } else {
        state.swd_chip_name.as_str()
    };
    let file_state = if state.swd_file_path.is_empty() {
        "No file selected"
    } else {
        "File ready"
    };

    let panel = Paragraph::new(vec![
        setup_cycle_line(
            "Probe",
            &probe_val,
            state.swd_field == SwdConfigField::Probe,
            swd_field_style(state, active, SwdConfigField::Probe),
        ),
        setup_cycle_line(
            "Speed",
            &format!("{} kHz", state.swd_speed_khz()),
            state.swd_field == SwdConfigField::Speed,
            swd_field_style(state, active, SwdConfigField::Speed),
        ),
        setup_cycle_line(
            "Mode",
            state.swd_connect_mode.label(),
            state.swd_field == SwdConfigField::ConnectMode,
            swd_field_style(state, active, SwdConfigField::ConnectMode),
        ),
        setup_cycle_line(
            "Verify",
            verify,
            state.swd_field == SwdConfigField::Verify,
            swd_field_style(state, active, SwdConfigField::Verify),
        ),
        setup_cycle_line(
            "Reset",
            reset,
            state.swd_field == SwdConfigField::ResetRun,
            swd_field_style(state, active, SwdConfigField::ResetRun),
        ),
        setup_value_line(
            "Base",
            &state.swd_bin_base_address,
            state.swd_field == SwdConfigField::BinBaseAddress,
            swd_field_style(state, active, SwdConfigField::BinBaseAddress),
        ),
        setup_cycle_line(
            "Preset",
            state.swd_chip_preset(),
            state.swd_field == SwdConfigField::ChipPreset,
            swd_field_style(state, active, SwdConfigField::ChipPreset),
        ),
        setup_value_line(
            "Chip",
            chip,
            state.swd_field == SwdConfigField::ChipName,
            swd_field_style(state, active, SwdConfigField::ChipName),
        ),
        setup_value_line(
            "File",
            file_state,
            state.swd_field == SwdConfigField::FilePath,
            swd_field_style(state, active, SwdConfigField::FilePath),
        ),
        Line::from(""),
        Line::from(Span::styled("Notes", theme::title_style())),
        Line::from(Span::styled(
            "  Use Under Reset if attach fails",
            Style::default().fg(theme::WARNING),
        )),
        Line::from(Span::styled(
            "  BIN files use Base address",
            theme::muted_style(),
        )),
        Line::from(Span::styled(
            "  Verify checks flash after write",
            theme::muted_style(),
        )),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(panel, inner);
}

fn setup_cycle_line(
    label: &'static str,
    value: &str,
    focused: bool,
    style: Style,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<7}", label), theme::title_style()),
        Span::styled(if focused { "◄ ► " } else { "" }, style),
        Span::styled(value.to_string(), style),
    ])
}

fn setup_value_line(
    label: &'static str,
    value: &str,
    focused: bool,
    style: Style,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<7}", label), theme::title_style()),
        Span::styled(if focused { "▌ " } else { "" }, style),
        Span::styled(value.to_string(), style),
    ])
}

fn jtag_field_style(state: &FlasherState, active: bool, field: JtagConfigField) -> Style {
    if !active {
        theme::muted_style()
    } else if state.jtag_field == field {
        theme::selected_style()
    } else {
        Style::default()
    }
}

fn swd_field_style(state: &FlasherState, active: bool, field: SwdConfigField) -> Style {
    if !active {
        theme::muted_style()
    } else if state.swd_field == field {
        theme::selected_style()
    } else {
        Style::default()
    }
}

fn render_isp_file_input(frame: &mut Frame, state: &FlasherState, area: Rect) {
    render_firmware_file_input(
        frame,
        state,
        area,
        &state.isp_file_path,
        state.isp_file_cursor,
        state.sub_screen == FlasherSubScreen::Config && state.isp_field == IspConfigField::FilePath,
    );
}

fn render_firmware_file_input(
    frame: &mut Frame,
    state: &FlasherState,
    area: Rect,
    file_path: &str,
    file_cursor: usize,
    focused: bool,
) {
    let active = state.sub_screen == FlasherSubScreen::Config;
    let border_color = if focused { theme::ACCENT } else { theme::MUTED };
    let block = Block::new()
        .title(" Firmware File (.bin/.hex) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let prefix = "  ";
    let prefix_width = UnicodeWidthStr::width(prefix) as u16;
    let value_width = inner.width.saturating_sub(prefix_width).saturating_sub(1);
    let (value, cursor_col) = clipped_text_with_cursor(file_path, file_cursor, value_width);

    let value_style = if file_path.is_empty() {
        theme::muted_style()
    } else if focused {
        theme::selected_style()
    } else if active {
        Style::default()
    } else {
        theme::muted_style()
    };
    let display_value = if file_path.is_empty() {
        "Type firmware path here".to_string()
    } else {
        value
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(prefix),
            Span::styled(display_value, value_style),
        ])),
        inner,
    );

    if focused {
        let x = inner
            .x
            .saturating_add(prefix_width)
            .saturating_add(cursor_col)
            .min(inner.x + inner.width.saturating_sub(1));
        frame.set_cursor_position((x, inner.y));
    }
}

fn isp_field_style(state: &FlasherState, active: bool, field: IspConfigField) -> Style {
    if !active {
        theme::muted_style()
    } else if state.isp_field == field {
        theme::selected_style()
    } else {
        Style::default()
    }
}

fn clipped_text_with_cursor(text: &str, cursor: usize, max_width: u16) -> (String, u16) {
    if max_width == 0 {
        return (String::new(), 0);
    }

    let cursor = cursor.min(text.len());
    let total_width = UnicodeWidthStr::width(text);
    let max_width = max_width as usize;
    if total_width <= max_width {
        return (
            text.to_string(),
            UnicodeWidthStr::width(&text[..cursor]) as u16,
        );
    }

    if max_width <= 3 {
        return (".".repeat(max_width), 0);
    }

    let cursor_width = UnicodeWidthStr::width(&text[..cursor]);
    let content_width = max_width - 3;

    if cursor_width <= content_width {
        let mut visible = take_width(text, content_width);
        visible.push_str("...");
        return (visible, cursor_width as u16);
    }

    let start_width = cursor_width.saturating_sub(content_width);
    let start = byte_index_at_width(text, start_width);
    let tail = take_width(&text[start..], content_width);
    let cursor_col = 3 + UnicodeWidthStr::width(&text[start..cursor]);
    (format!("...{}", tail), cursor_col as u16)
}

fn take_width(text: &str, max_width: usize) -> String {
    let mut used = 0;
    let mut out = String::new();
    for ch in text.chars() {
        let width = ch.width().unwrap_or(0);
        if used + width > max_width {
            break;
        }
        used += width;
        out.push(ch);
    }
    out
}

fn byte_index_at_width(text: &str, target_width: usize) -> usize {
    let mut used = 0;
    for (idx, ch) in text.char_indices() {
        if used >= target_width {
            return idx;
        }
        used += ch.width().unwrap_or(0);
    }
    text.len()
}

fn render_operation_log(
    frame: &mut Frame,
    state: &FlasherState,
    area: ratatui::layout::Rect,
    title: &'static str,
) {
    let log_block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::MUTED));
    let log_inner = log_block.inner(area);
    state.log_visible_rows.set(log_inner.height);
    let visible = log_inner.height as usize;

    let log_lines: Vec<Line<'static>> = if state.log.is_empty() {
        vec![Line::from(Span::styled(
            "  No operation yet. Press Enter to start flashing.",
            theme::muted_style(),
        ))]
    } else {
        state
            .log
            .iter()
            .map(|s| {
                Line::from(Span::styled(
                    format!("  {}", s),
                    Style::default().fg(Color::Gray),
                ))
            })
            .collect()
    };

    let total = log_lines.len();
    let scroll = state.log_scroll.min(total.saturating_sub(visible)) as u16;

    frame.render_widget(
        Paragraph::new(log_lines)
            .block(log_block)
            .scroll((scroll, 0)),
        area,
    );

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

fn render_progress_gauge(frame: &mut Frame, state: &FlasherState, area: ratatui::layout::Rect) {
    let (pct, gauge_label, gauge_color) = if state.sub_screen == FlasherSubScreen::Config {
        (0u16, " Ready ".to_string(), theme::ACCENT)
    } else if state.op_done {
        if state.op_ok {
            (100u16, " Complete ".to_string(), theme::SUCCESS)
        } else {
            (
                state.progress_pct.unwrap_or(0) as u16,
                " Failed ".to_string(),
                theme::DANGER,
            )
        }
    } else {
        let p = state.progress_pct.unwrap_or(0) as u16;
        (p, format!(" {}% ", p), theme::WARNING)
    };

    let gauge = Gauge::default()
        .block(
            Block::new()
                .title(" Progress ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(gauge_color)),
        )
        .gauge_style(Style::default().fg(gauge_color).bg(theme::MUTED))
        .label(gauge_label)
        .percent(pct);
    frame.render_widget(gauge, area);
}
