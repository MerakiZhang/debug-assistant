use crate::features::protocols::flashing::state::{
    FlasherState, FlasherSubScreen, JtagConfigField,
};
use crate::features::protocols::flashing::ui;
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

    ui::render_workbench_title(frame, title_area, state, "JTAG Workbench");
    let [setup_area, log_area] =
        Layout::horizontal([Constraint::Length(34), Constraint::Fill(1)]).areas(body_area);
    render_setup_panel(frame, state, setup_area);
    ui::render_firmware_file_input(
        frame,
        state,
        file_area,
        &state.jtag.file_path,
        state.jtag.file_cursor,
        state.sub_screen == FlasherSubScreen::Config
            && state.jtag.field == JtagConfigField::FilePath,
    );
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
    }
}

fn render_setup_panel(frame: &mut Frame, state: &FlasherState, area: Rect) {
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

    let probe_val = if state.jtag.probe_list.is_empty() {
        "(no probes found)".to_string()
    } else {
        state.jtag.probe_list[state.jtag.probe_idx].clone()
    };
    let verify = if state.jtag.verify {
        "Enabled"
    } else {
        "Disabled"
    };
    let reset = if state.jtag.reset_run {
        "Enabled"
    } else {
        "Disabled"
    };
    let chip = if state.jtag.chip_name.is_empty() {
        "(required)"
    } else {
        state.jtag.chip_name.as_str()
    };
    let file_state = if state.jtag.file_path.is_empty() {
        "No file selected"
    } else {
        "File ready"
    };

    let panel = Paragraph::new(vec![
        ui::setup_cycle_line(
            "Probe",
            &probe_val,
            state.jtag.field == JtagConfigField::Probe,
            field_style(state, active, JtagConfigField::Probe),
        ),
        ui::setup_cycle_line(
            "Speed",
            &format!("{} kHz", state.jtag_speed_khz()),
            state.jtag.field == JtagConfigField::Speed,
            field_style(state, active, JtagConfigField::Speed),
        ),
        ui::setup_cycle_line(
            "Verify",
            verify,
            state.jtag.field == JtagConfigField::Verify,
            field_style(state, active, JtagConfigField::Verify),
        ),
        ui::setup_cycle_line(
            "Reset",
            reset,
            state.jtag.field == JtagConfigField::ResetRun,
            field_style(state, active, JtagConfigField::ResetRun),
        ),
        ui::setup_value_line(
            "Base",
            &state.jtag.bin_base_address,
            state.jtag.field == JtagConfigField::BinBaseAddress,
            field_style(state, active, JtagConfigField::BinBaseAddress),
        ),
        ui::setup_cycle_line(
            "Preset",
            state.jtag_chip_preset(),
            state.jtag.field == JtagConfigField::ChipPreset,
            field_style(state, active, JtagConfigField::ChipPreset),
        ),
        ui::setup_value_line(
            "Chip",
            chip,
            state.jtag.field == JtagConfigField::ChipName,
            field_style(state, active, JtagConfigField::ChipName),
        ),
        ui::setup_value_line(
            "File",
            file_state,
            state.jtag.field == JtagConfigField::FilePath,
            field_style(state, active, JtagConfigField::FilePath),
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

fn field_style(state: &FlasherState, active: bool, field: JtagConfigField) -> Style {
    if !active {
        theme::muted_style()
    } else if state.jtag.field == field {
        theme::selected_style()
    } else {
        Style::default()
    }
}
