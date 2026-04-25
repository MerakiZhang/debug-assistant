use super::state::{
    FlasherMethod, FlasherState, FlasherSubScreen, IspBootMode, IspConfigField, JtagConfigField,
    METHOD_ITEMS, SwdConfigField,
};
use crate::serial::ISP_BAUD_PRESETS;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

pub fn render(frame: &mut Frame, state: &FlasherState) {
    match state.sub_screen {
        FlasherSubScreen::MethodSelect => render_method_select(frame, state),
        FlasherSubScreen::Config => render_config(frame, state),
        FlasherSubScreen::Progress => render_progress(frame, state),
    }
}

fn render_method_select(frame: &mut Frame, state: &FlasherState) {
    let area = frame.area();

    let [title_area, body_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    frame.render_widget(
        Paragraph::new(" STM32 Flasher — Download Tool ")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        title_area,
    );

    let [_, center_col, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(36),
        Constraint::Fill(1),
    ])
    .areas(body_area);

    let [_, list_area, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(METHOD_ITEMS.len() as u16 + 2),
        Constraint::Fill(1),
    ])
    .areas(center_col);

    let list_block = Block::new()
        .title(" Select Download Method ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let list_inner = list_block.inner(list_area);
    frame.render_widget(list_block, list_area);

    let row_constraints: Vec<Constraint> =
        METHOD_ITEMS.iter().map(|_| Constraint::Length(1)).collect();
    let rows = Layout::vertical(row_constraints).split(list_inner);

    for (i, item) in METHOD_ITEMS.iter().enumerate() {
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
            rows[i],
        );
    }

    frame.render_widget(
        Paragraph::new("  ↑↓:Navigate  Enter:Select  Esc:Back to Home")
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

fn render_config(frame: &mut Frame, state: &FlasherState) {
    match state.method {
        FlasherMethod::UsartIsp => render_isp_config(frame, state),
        FlasherMethod::Jtag => render_jtag_config(frame, state),
        FlasherMethod::Swd => render_swd_config(frame, state),
    }
}

fn render_isp_config(frame: &mut Frame, state: &FlasherState) {
    let area = frame.area();

    let [title_area, body_area, note_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(9),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    frame.render_widget(
        Paragraph::new(" STM32 Flasher — USART ISP Configuration ")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        title_area,
    );

    let config_block = Block::new()
        .title(" Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let config_inner = config_block.inner(body_area);
    frame.render_widget(config_block, body_area);

    let port_val = if state.isp_port_list.is_empty() {
        "(none)".to_string()
    } else {
        state.isp_port_list[state.isp_port_idx].clone()
    };

    let fields: &[(IspConfigField, &str, String, bool)] = &[
        (IspConfigField::Port, "Port       ", port_val, false),
        (
            IspConfigField::BaudRate,
            "Baud Rate  ",
            ISP_BAUD_PRESETS[state.isp_baud_idx].to_string(),
            false,
        ),
        (
            IspConfigField::BootMode,
            "Boot Mode  ",
            state.isp_boot_mode.label().to_string(),
            false,
        ),
        (
            IspConfigField::AutoProfile,
            "Auto Mode  ",
            state.isp_auto_profile.label().to_string(),
            false,
        ),
        (
            IspConfigField::FilePath,
            "File (.bin/.hex)",
            state.isp_file_path.clone(),
            true,
        ),
    ];

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(config_inner);

    for (i, (field, label, value, is_text)) in fields.iter().enumerate() {
        let focused = state.isp_field == *field;
        let inactive = *field == IspConfigField::AutoProfile && state.isp_boot_mode == IspBootMode::Manual;
        let (style, arrow) = if inactive {
            (Style::default().fg(Color::DarkGray), "    ")
        } else if focused {
            (
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                if *is_text { "  " } else { "◄ ► " },
            )
        } else {
            (Style::default(), if *is_text { "  " } else { "    " })
        };
        let text = format!("  {}: {}{}", label, arrow, value);
        frame.render_widget(Paragraph::new(text).style(style), rows[i]);
    }

    let note = Paragraph::new(vec![
        Line::from(""),
        if state.isp_boot_mode == IspBootMode::Manual {
            Line::from(Span::styled(
                "  NOTE: Device must be in STM32 bootloader mode before flashing.",
                Style::default().fg(Color::Yellow),
            ))
        } else {
            Line::from(Span::styled(
                "  NOTE: Auto mode drives BOOT0/RESET through RTS/DTR.",
                Style::default().fg(Color::Yellow),
            ))
        },
        if state.isp_boot_mode == IspBootMode::Manual {
            Line::from(Span::styled(
                "  (Set BOOT0 pin HIGH, then reset the board)",
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            Line::from(Span::styled(
                "  Standard fits common CH340/CP210x inverter circuits; try Inverted if sync fails.",
                Style::default().fg(Color::DarkGray),
            ))
        },
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(note, note_area);

    frame.render_widget(
        Paragraph::new(
            "  ↑↓/Tab:Field  ←→:Change option  Type:File path  Enter:Start  Esc:Back",
        )
        .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

fn render_jtag_config(frame: &mut Frame, state: &FlasherState) {
    let area = frame.area();

    let [title_area, body_area, note_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(7),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    frame.render_widget(
        Paragraph::new(" STM32 Flasher — JTAG Configuration ")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        title_area,
    );

    let config_block = Block::new()
        .title(" Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let config_inner = config_block.inner(body_area);
    frame.render_widget(config_block, body_area);

    let probe_val = if state.jtag_probe_list.is_empty() {
        "(no probes found)".to_string()
    } else {
        state.jtag_probe_list[state.jtag_probe_idx].clone()
    };

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(config_inner);

    render_probe_field(
        frame,
        rows[0],
        state.jtag_field == JtagConfigField::Probe,
        &probe_val,
    );

    render_text_field(
        frame,
        rows[1],
        state.jtag_field == JtagConfigField::ChipName,
        "  Chip Name  :   ",
        &state.jtag_chip_name,
    );

    render_text_field(
        frame,
        rows[2],
        state.jtag_field == JtagConfigField::FilePath,
        "  File (.bin/.hex):   ",
        &state.jtag_file_path,
    );

    let note = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  NOTE: JTAG currently provides basic probe flashing only.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  Press R to refresh probes. Install probe drivers before use.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(note, note_area);

    frame.render_widget(
        Paragraph::new("  ↑↓/Tab:Field  ←→:Change Probe  Type:Chip/File  R:Refresh  Enter:Start  Esc:Back")
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

fn render_swd_config(frame: &mut Frame, state: &FlasherState) {
    let area = frame.area();

    let [title_area, body_area, note_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(13),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    frame.render_widget(
        Paragraph::new(" STM32 Flasher — SWD Configuration ")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        title_area,
    );

    let config_block = Block::new()
        .title(" Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let config_inner = config_block.inner(body_area);
    frame.render_widget(config_block, body_area);

    let probe_val = if state.swd_probe_list.is_empty() {
        "(no probes found)".to_string()
    } else {
        state.swd_probe_list[state.swd_probe_idx].clone()
    };

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(config_inner);

    render_cycle_field(
        frame,
        rows[0],
        state.swd_field == SwdConfigField::Probe,
        "Probe          ",
        &probe_val,
    );
    render_cycle_field(
        frame,
        rows[1],
        state.swd_field == SwdConfigField::Speed,
        "Speed (kHz)    ",
        &state.swd_speed_khz().to_string(),
    );
    render_cycle_field(
        frame,
        rows[2],
        state.swd_field == SwdConfigField::ConnectMode,
        "Connect Mode   ",
        state.swd_connect_mode.label(),
    );
    render_cycle_field(
        frame,
        rows[3],
        state.swd_field == SwdConfigField::Verify,
        "Verify         ",
        if state.swd_verify { "Enabled" } else { "Disabled" },
    );
    render_cycle_field(
        frame,
        rows[4],
        state.swd_field == SwdConfigField::ResetRun,
        "Reset + Run    ",
        if state.swd_reset_run { "Enabled" } else { "Disabled" },
    );
    render_text_field(
        frame,
        rows[5],
        state.swd_field == SwdConfigField::BinBaseAddress,
        "  BIN Base Addr :   ",
        &state.swd_bin_base_address,
    );
    render_cycle_field(
        frame,
        rows[6],
        state.swd_field == SwdConfigField::ChipPreset,
        "Chip Preset    ",
        state.swd_chip_preset(),
    );
    render_text_field(
        frame,
        rows[7],
        state.swd_field == SwdConfigField::ChipName,
        "  Chip Name     :   ",
        &state.swd_chip_name,
    );
    render_text_field(
        frame,
        rows[8],
        state.swd_field == SwdConfigField::FilePath,
        "  File (.bin/.hex): ",
        &state.swd_file_path,
    );

    let note = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  NOTE: Use Under Reset when the target is hard to attach or remaps SWD pins.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  BIN files need a flash base address; presets fill Chip Name and manual editing is still supported.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(note, note_area);

    frame.render_widget(
        Paragraph::new(
            "  ↑↓/Tab:Field  ←→:Change option  Type:Addr/Chip/File  R:Refresh  Enter:Start  Esc:Back",
        )
        .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

fn render_probe_field(frame: &mut Frame, area: ratatui::layout::Rect, focused: bool, value: &str) {
    let (style, arrow) = if focused {
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
    frame.render_widget(
        Paragraph::new(format!("  Probe      : {}{}", arrow, value)).style(style),
        area,
    );
}

fn render_cycle_field(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    focused: bool,
    label: &str,
    value: &str,
) {
    let (style, arrow) = if focused {
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
    frame.render_widget(
        Paragraph::new(format!("  {}: {}{}", label, arrow, value)).style(style),
        area,
    );
}

fn render_text_field(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    focused: bool,
    prefix: &str,
    value: &str,
) {
    let style = if focused {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    frame.render_widget(Paragraph::new(format!("{}{}", prefix, value)).style(style), area);
}

fn render_progress(frame: &mut Frame, state: &FlasherState) {
    let area = frame.area();

    let [title_area, log_area, gauge_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(area);

    let title_label = match state.method {
        FlasherMethod::UsartIsp => " STM32 Flasher — USART ISP Progress ",
        FlasherMethod::Jtag => " STM32 Flasher — JTAG Progress ",
        FlasherMethod::Swd => " STM32 Flasher — SWD Progress ",
    };
    let title_color = if state.op_done {
        if state.op_ok {
            Color::Green
        } else {
            Color::Red
        }
    } else {
        Color::Yellow
    };
    frame.render_widget(
        Paragraph::new(title_label)
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
        title_area,
    );

    let log_block = Block::new()
        .title(" Operation Log ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let log_inner = log_block.inner(log_area);
    state.log_visible_rows.set(log_inner.height);
    let visible = log_inner.height as usize;

    let log_lines: Vec<Line<'static>> = state
        .log
        .iter()
        .map(|s| {
            Line::from(Span::styled(
                format!("  {}", s),
                Style::default().fg(Color::Gray),
            ))
        })
        .collect();

    let total = log_lines.len();
    let scroll = state.log_scroll.min(total.saturating_sub(visible)) as u16;

    frame.render_widget(
        Paragraph::new(log_lines)
            .block(log_block)
            .scroll((scroll, 0)),
        log_area,
    );

    if total > visible {
        let mut sb = ScrollbarState::new(total.saturating_sub(visible)).position(scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            log_area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut sb,
        );
    }

    let (pct, gauge_label, gauge_color) = if state.op_done {
        if state.op_ok {
            (100u16, " Complete ".to_string(), Color::Green)
        } else {
            (
                state.progress_pct.unwrap_or(0) as u16,
                " Failed ".to_string(),
                Color::Red,
            )
        }
    } else {
        let p = state.progress_pct.unwrap_or(0) as u16;
        (p, format!(" {}% ", p), Color::Yellow)
    };

    let gauge = Gauge::default()
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(gauge_color)),
        )
        .gauge_style(Style::default().fg(gauge_color).bg(Color::DarkGray))
        .label(gauge_label)
        .percent(pct);
    frame.render_widget(gauge, gauge_area);

    let hint = if state.op_done {
        "  Up/Down/PgUp/PgDn/Home/End: Scroll log  Esc: Back to config"
    } else if state.cancel_armed {
        "  Up/Down/PgUp/PgDn/Home/End: Scroll log  Esc: Confirm cancel"
    } else {
        "  Up/Down/PgUp/PgDn/Home/End: Scroll log  Esc: Arm cancel"
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}
