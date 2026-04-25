pub mod common;
pub mod jtag;
pub mod state;
pub mod swd;
pub mod ui;
pub mod usart_isp;

pub use state::FlasherState;
use state::{
    FlasherMethod, FlasherSubScreen, IspBootMode, IspConfigField, JtagConfigField,
    METHOD_ITEMS, SwdConfigField, SWD_CHIP_PRESETS, SWD_SPEED_PRESETS,
};

use crate::event::AppEvent;
use crate::serial_monitor::SerialMonitorState;
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    GoHome,
    Quit,
}

pub fn render(frame: &mut ratatui::Frame, state: &FlasherState) {
    ui::render(frame, state);
}

pub fn handle_key(
    state: &mut FlasherState,
    code: KeyCode,
    mods: KeyModifiers,
    serial_monitor: Option<&mut SerialMonitorState>,
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    match state.sub_screen {
        FlasherSubScreen::MethodSelect => handle_method_select(state, code),
        FlasherSubScreen::Config => handle_config(state, code, mods, serial_monitor, tx),
        FlasherSubScreen::Progress => handle_progress(state, code),
    }
}

fn handle_method_select(state: &mut FlasherState, code: KeyCode) -> Action {
    match code {
        KeyCode::Esc => Action::GoHome,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Up => {
            if state.selected == 0 {
                state.selected = METHOD_ITEMS.len() - 1;
            } else {
                state.selected -= 1;
            }
            Action::None
        }
        KeyCode::Down => {
            state.selected = (state.selected + 1) % METHOD_ITEMS.len();
            Action::None
        }
        KeyCode::Enter => {
            state.enter_config();
            Action::None
        }
        _ => Action::None,
    }
}

fn handle_config(
    state: &mut FlasherState,
    code: KeyCode,
    mods: KeyModifiers,
    serial_monitor: Option<&mut SerialMonitorState>,
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    match state.method {
        FlasherMethod::UsartIsp => handle_isp_config(state, code, mods, serial_monitor, tx),
        FlasherMethod::Jtag => handle_jtag_config(state, code, mods, tx),
        FlasherMethod::Swd => handle_swd_config(state, code, mods, tx),
    }
}

fn handle_isp_config(
    state: &mut FlasherState,
    code: KeyCode,
    mods: KeyModifiers,
    serial_monitor: Option<&mut SerialMonitorState>,
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    match (code, mods) {
        (KeyCode::Esc, _) => {
            state.sub_screen = FlasherSubScreen::MethodSelect;
            Action::None
        }
        (KeyCode::Char('q'), KeyModifiers::NONE) => Action::Quit,
        (KeyCode::Up, _) | (KeyCode::BackTab, _) => {
            state.isp_field_prev();
            Action::None
        }
        (KeyCode::Down, _) | (KeyCode::Tab, KeyModifiers::NONE) => {
            state.isp_field_next();
            Action::None
        }
        (KeyCode::Left, _) => {
            cycle_isp_option(state, false);
            Action::None
        }
        (KeyCode::Right, _) => {
            cycle_isp_option(state, true);
            Action::None
        }
        (KeyCode::Backspace, _) if state.isp_field == IspConfigField::FilePath => {
            state.isp_file_backspace();
            Action::None
        }
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT)
            if state.isp_field == IspConfigField::FilePath =>
        {
            state.isp_file_input_char(c);
            Action::None
        }
        (KeyCode::Enter, _) => {
            let mut preflight_log = None;
            state.clear_serial_monitor_restore();
            if let Some(port_name) = selected_isp_port_name(state).map(str::to_string) {
                if let Some(serial_monitor) = serial_monitor {
                    if serial_monitor.connected && serial_monitor.serial_config.port_name == port_name {
                        state.plan_serial_monitor_restore(serial_monitor.serial_config.clone());
                        serial_monitor.disconnect();
                        preflight_log = Some(format!(
                            "Disconnected serial monitor from {} before flashing.",
                            port_name
                        ));
                    }
                }
            }

            state.enter_progress();
            if let Some(message) = preflight_log {
                state.log.push(message);
            }
            if let Err(e) = usart_isp::start_flash(state, tx) {
                state.log.push(format!("Error: {}", e));
                state.op_done = true;
                state.op_ok = false;
            }
            Action::None
        }
        _ => Action::None,
    }
}

fn selected_isp_port_name(state: &FlasherState) -> Option<&str> {
    state
        .isp_port_list
        .get(state.isp_port_idx)
        .map(String::as_str)
        .filter(|name| *name != "(no ports found)")
}

fn cycle_isp_option(state: &mut FlasherState, forward: bool) {
    match state.isp_field {
        IspConfigField::Port => {
            if state.isp_port_list.is_empty() {
                return;
            }
            if forward {
                state.isp_port_idx = (state.isp_port_idx + 1) % state.isp_port_list.len();
            } else if state.isp_port_idx == 0 {
                state.isp_port_idx = state.isp_port_list.len() - 1;
            } else {
                state.isp_port_idx -= 1;
            }
        }
        IspConfigField::BaudRate => {
            if forward {
                state.isp_baud_idx = (state.isp_baud_idx + 1) % crate::serial::ISP_BAUD_PRESETS.len();
            } else if state.isp_baud_idx == 0 {
                state.isp_baud_idx = crate::serial::ISP_BAUD_PRESETS.len() - 1;
            } else {
                state.isp_baud_idx -= 1;
            }
        }
        IspConfigField::BootMode => {
            state.isp_boot_mode = if forward {
                state.isp_boot_mode.next()
            } else {
                state.isp_boot_mode.prev()
            };
        }
        IspConfigField::AutoProfile => {
            if state.isp_boot_mode == IspBootMode::Auto {
                state.isp_auto_profile = if forward {
                    state.isp_auto_profile.next()
                } else {
                    state.isp_auto_profile.prev()
                };
            }
        }
        IspConfigField::FilePath => {}
    }
}

fn handle_jtag_config(
    state: &mut FlasherState,
    code: KeyCode,
    mods: KeyModifiers,
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    match (code, mods) {
        (KeyCode::Esc, _) => {
            state.sub_screen = FlasherSubScreen::MethodSelect;
            Action::None
        }
        (KeyCode::Char('r'), KeyModifiers::NONE) => {
            if state.jtag_field == JtagConfigField::Probe {
                state.refresh_jtag_probes();
                Action::None
            } else {
                match state.jtag_field {
                    JtagConfigField::ChipName => state.jtag_chip_input_char('r'),
                    JtagConfigField::FilePath => state.jtag_file_input_char('r'),
                    JtagConfigField::Probe => {}
                }
                Action::None
            }
        }
        (KeyCode::Char('q'), KeyModifiers::NONE) if state.jtag_field == JtagConfigField::Probe => {
            Action::Quit
        }
        (KeyCode::Up, _) | (KeyCode::BackTab, _) => {
            state.jtag_field = state.jtag_field.prev();
            Action::None
        }
        (KeyCode::Down, _) | (KeyCode::Tab, KeyModifiers::NONE) => {
            state.jtag_field = state.jtag_field.next();
            Action::None
        }
        (KeyCode::Left, _) if state.jtag_field == JtagConfigField::Probe => {
            cycle_probe_idx(&mut state.jtag_probe_idx, state.jtag_probe_list.len(), false);
            Action::None
        }
        (KeyCode::Right, _) if state.jtag_field == JtagConfigField::Probe => {
            cycle_probe_idx(&mut state.jtag_probe_idx, state.jtag_probe_list.len(), true);
            Action::None
        }
        (KeyCode::Backspace, _) => {
            match state.jtag_field {
                JtagConfigField::ChipName => state.jtag_chip_backspace(),
                JtagConfigField::FilePath => state.jtag_file_backspace(),
                JtagConfigField::Probe => {}
            }
            Action::None
        }
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            match state.jtag_field {
                JtagConfigField::ChipName => state.jtag_chip_input_char(c),
                JtagConfigField::FilePath => state.jtag_file_input_char(c),
                JtagConfigField::Probe => {}
            }
            Action::None
        }
        (KeyCode::Enter, _) => {
            state.enter_progress();
            if let Err(e) = jtag::start_flash(state, tx) {
                state.log.push(format!("Error: {}", e));
                state.op_done = true;
                state.op_ok = false;
            }
            Action::None
        }
        _ => Action::None,
    }
}

fn handle_swd_config(
    state: &mut FlasherState,
    code: KeyCode,
    mods: KeyModifiers,
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    match (code, mods) {
        (KeyCode::Esc, _) => {
            state.sub_screen = FlasherSubScreen::MethodSelect;
            Action::None
        }
        (KeyCode::Char('r'), KeyModifiers::NONE) => {
            if state.swd_field == SwdConfigField::Probe {
                state.refresh_swd_probes();
                Action::None
            } else {
                match state.swd_field {
                    SwdConfigField::BinBaseAddress => state.swd_bin_base_input_char('r'),
                    SwdConfigField::ChipName => state.swd_chip_input_char('r'),
                    SwdConfigField::FilePath => state.swd_file_input_char('r'),
                    _ => {}
                }
                Action::None
            }
        }
        (KeyCode::Char('q'), KeyModifiers::NONE)
            if !matches!(
                state.swd_field,
                SwdConfigField::BinBaseAddress | SwdConfigField::ChipName | SwdConfigField::FilePath
            ) =>
        {
            Action::Quit
        }
        (KeyCode::Up, _) | (KeyCode::BackTab, _) => {
            state.swd_field = state.swd_field.prev();
            Action::None
        }
        (KeyCode::Down, _) | (KeyCode::Tab, KeyModifiers::NONE) => {
            state.swd_field = state.swd_field.next();
            Action::None
        }
        (KeyCode::Left, _) => {
            cycle_swd_option(state, false);
            Action::None
        }
        (KeyCode::Right, _) => {
            cycle_swd_option(state, true);
            Action::None
        }
        (KeyCode::Backspace, _) => {
                match state.swd_field {
                    SwdConfigField::BinBaseAddress => state.swd_bin_base_backspace(),
                    SwdConfigField::ChipName => state.swd_chip_backspace(),
                    SwdConfigField::FilePath => state.swd_file_backspace(),
                    SwdConfigField::ChipPreset => {}
                    _ => {}
                }
            Action::None
        }
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                match state.swd_field {
                    SwdConfigField::BinBaseAddress => state.swd_bin_base_input_char(c),
                    SwdConfigField::ChipName => state.swd_chip_input_char(c),
                    SwdConfigField::FilePath => state.swd_file_input_char(c),
                    SwdConfigField::ChipPreset => {}
                    _ => {}
                }
            Action::None
        }
        (KeyCode::Enter, _) => {
            state.enter_progress();
            if let Err(e) = swd::start_flash(state, tx) {
                state.log.push(format!("Error: {}", e));
                state.op_done = true;
                state.op_ok = false;
            }
            Action::None
        }
        _ => Action::None,
    }
}

fn cycle_probe_idx(idx: &mut usize, len: usize, forward: bool) {
    if len == 0 {
        return;
    }

    if forward {
        *idx = (*idx + 1) % len;
    } else if *idx == 0 {
        *idx = len - 1;
    } else {
        *idx -= 1;
    }
}

fn cycle_swd_option(state: &mut FlasherState, forward: bool) {
    match state.swd_field {
        SwdConfigField::Probe => {
            cycle_probe_idx(&mut state.swd_probe_idx, state.swd_probe_list.len(), forward)
        }
        SwdConfigField::Speed => {
            if forward {
                state.swd_speed_idx = (state.swd_speed_idx + 1) % SWD_SPEED_PRESETS.len();
            } else if state.swd_speed_idx == 0 {
                state.swd_speed_idx = SWD_SPEED_PRESETS.len() - 1;
            } else {
                state.swd_speed_idx -= 1;
            }
        }
        SwdConfigField::ConnectMode => {
            state.swd_connect_mode = if forward {
                state.swd_connect_mode.next()
            } else {
                state.swd_connect_mode.prev()
            };
        }
        SwdConfigField::Verify => state.swd_verify = !state.swd_verify,
        SwdConfigField::ResetRun => state.swd_reset_run = !state.swd_reset_run,
        SwdConfigField::ChipPreset => {
            if forward {
                state.swd_chip_preset_idx = (state.swd_chip_preset_idx + 1) % SWD_CHIP_PRESETS.len();
            } else if state.swd_chip_preset_idx == 0 {
                state.swd_chip_preset_idx = SWD_CHIP_PRESETS.len() - 1;
            } else {
                state.swd_chip_preset_idx -= 1;
            }
            state.swd_apply_chip_preset();
        }
        SwdConfigField::BinBaseAddress | SwdConfigField::ChipName | SwdConfigField::FilePath => {}
    }
}

fn handle_progress(state: &mut FlasherState, code: KeyCode) -> Action {
    let visible = state.log_visible_rows.get() as usize;
    match code {
        KeyCode::Esc => {
            if state.op_done {
                state.cancel_armed = false;
                state.sub_screen = FlasherSubScreen::Config;
                return Action::None;
            }

            if !state.cancel_armed {
                state.cancel_armed = true;
                state.log.push("Press Esc again to cancel flashing.".into());
                return Action::None;
            }

            if let Some(flag) = state.stop_flag.take() {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            state.cancel_armed = false;
            state.sub_screen = FlasherSubScreen::Config;
            Action::None
        }
        KeyCode::Char('q') => {
            if let Some(flag) = state.stop_flag.take() {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            state.cancel_armed = false;
            Action::Quit
        }
        KeyCode::Up => {
            state.cancel_armed = false;
            state.scroll_log_up(1);
            Action::None
        }
        KeyCode::Down => {
            state.cancel_armed = false;
            state.scroll_log_down(1);
            Action::None
        }
        KeyCode::PageUp => {
            state.cancel_armed = false;
            state.scroll_log_up(visible.saturating_sub(1));
            Action::None
        }
        KeyCode::PageDown => {
            state.cancel_armed = false;
            state.scroll_log_down(visible.saturating_sub(1));
            Action::None
        }
        KeyCode::Home => {
            state.cancel_armed = false;
            state.scroll_log_home();
            Action::None
        }
        KeyCode::End => {
            state.cancel_armed = false;
            state.scroll_log_end();
            Action::None
        }
        _ => {
            state.cancel_armed = false;
            Action::None
        }
    }
}
