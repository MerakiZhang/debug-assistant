pub mod jtag_swd;
pub mod state;
pub mod ui;
pub mod usart_isp;

pub use state::FlasherState;
use state::{FlasherMethod, FlasherSubScreen, IspBootMode, IspConfigField, JtagConfigField, METHOD_ITEMS};

use crate::event::AppEvent;
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
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    match state.sub_screen {
        FlasherSubScreen::MethodSelect => handle_method_select(state, code),
        FlasherSubScreen::Config => handle_config(state, code, mods, tx),
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
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    match state.method {
        FlasherMethod::UsartIsp => handle_isp_config(state, code, mods, tx),
        FlasherMethod::JtagSwd => handle_jtag_config(state, code, mods, tx),
    }
}

fn handle_isp_config(
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
            state.enter_progress();
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
        (KeyCode::Char('q'), KeyModifiers::NONE) => Action::Quit,
        (KeyCode::Up, _) | (KeyCode::BackTab, _) => {
            state.jtag_field = state.jtag_field.prev();
            Action::None
        }
        (KeyCode::Down, _) | (KeyCode::Tab, KeyModifiers::NONE) => {
            state.jtag_field = state.jtag_field.next();
            Action::None
        }
        (KeyCode::Left, _) if state.jtag_field == JtagConfigField::Probe => {
            if !state.jtag_probe_list.is_empty() {
                if state.jtag_probe_idx == 0 {
                    state.jtag_probe_idx = state.jtag_probe_list.len() - 1;
                } else {
                    state.jtag_probe_idx -= 1;
                }
            }
            Action::None
        }
        (KeyCode::Right, _) if state.jtag_field == JtagConfigField::Probe => {
            if !state.jtag_probe_list.is_empty() {
                state.jtag_probe_idx = (state.jtag_probe_idx + 1) % state.jtag_probe_list.len();
            }
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
            if let Err(e) = jtag_swd::start_flash(state, tx) {
                state.log.push(format!("Error: {}", e));
                state.op_done = true;
                state.op_ok = false;
            }
            Action::None
        }
        _ => Action::None,
    }
}

fn handle_progress(state: &mut FlasherState, code: KeyCode) -> Action {
    match code {
        KeyCode::Esc => {
            if let Some(flag) = state.stop_flag.take() {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            state.sub_screen = FlasherSubScreen::Config;
            Action::None
        }
        KeyCode::Char('q') => {
            if let Some(flag) = state.stop_flag.take() {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Action::Quit
        }
        KeyCode::Up => {
            state.log_scroll = state.log_scroll.saturating_sub(1);
            Action::None
        }
        KeyCode::Down => {
            if state.log_scroll + 1 < state.log.len() {
                state.log_scroll += 1;
            }
            Action::None
        }
        _ => Action::None,
    }
}
