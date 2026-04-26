use crate::features::protocols::flashing::state::{
    FlasherState, FlasherSubScreen, IspBootMode, IspConfigField,
};
use crate::features::protocols::flashing::ui;
use crate::transport::serial::ISP_BAUD_PRESETS;
use crate::ui::theme;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, state: &FlasherState) {
    let area = frame.area();
    let [title_area, body_area, file_area, gauge_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .areas(area);

    ui::render_workbench_title(frame, title_area, state, "Serial ISP Workbench");

    let [setup_area, log_area] =
        Layout::horizontal([Constraint::Length(32), Constraint::Fill(1)]).areas(body_area);
    render_setup_panel(frame, state, setup_area);
    render_file_input(frame, state, file_area);
    ui::render_operation_log(frame, state, log_area, " Operation Log ");
    ui::render_progress_gauge(frame, state, gauge_area);

    frame.render_widget(
        Paragraph::new(hint(state)).style(theme::muted_style()),
        hint_area,
    );
}

fn hint(state: &FlasherState) -> &'static str {
    match state.sub_screen {
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
    }
}

fn render_setup_panel(frame: &mut Frame, state: &FlasherState, area: Rect) {
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

    let port_val = if state.usart_isp.port_list.is_empty() {
        "(none)".to_string()
    } else {
        state.usart_isp.port_list[state.usart_isp.port_idx].clone()
    };

    let port_style = field_style(state, active, IspConfigField::Port);
    let baud_style = field_style(state, active, IspConfigField::BaudRate);
    let boot_style = field_style(state, active, IspConfigField::BootMode);
    let note_lines = if state.usart_isp.boot_mode == IspBootMode::Manual {
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
                if state.usart_isp.field == IspConfigField::Port {
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
                if state.usart_isp.field == IspConfigField::BaudRate {
                    "◄ ► "
                } else {
                    ""
                },
                baud_style,
            ),
            Span::styled(
                ISP_BAUD_PRESETS[state.usart_isp.baud_idx].to_string(),
                baud_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("Boot", theme::title_style()),
            Span::raw("  "),
            Span::styled(
                if state.usart_isp.field == IspConfigField::BootMode {
                    "◄ ► "
                } else {
                    ""
                },
                boot_style,
            ),
            Span::styled(state.usart_isp.boot_mode.label(), boot_style),
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

fn render_file_input(frame: &mut Frame, state: &FlasherState, area: Rect) {
    ui::render_firmware_file_input(
        frame,
        state,
        area,
        &state.usart_isp.file_path,
        state.usart_isp.file_cursor,
        state.sub_screen == FlasherSubScreen::Config
            && state.usart_isp.field == IspConfigField::FilePath,
    );
}

fn field_style(state: &FlasherState, active: bool, field: IspConfigField) -> Style {
    if !active {
        theme::muted_style()
    } else if state.usart_isp.field == field {
        theme::selected_style()
    } else {
        Style::default()
    }
}
