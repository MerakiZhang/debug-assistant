use std::sync::mpsc;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::event::AppEvent;
use crate::features::protocols::flashing::state::IspConfigField;
use crate::features::protocols::flashing::{Action, FlasherState};
use crate::features::protocols::uart::isp;
use crate::features::protocols::uart::monitor::SerialMonitorState;

pub fn handle_config(
    state: &mut FlasherState,
    code: KeyCode,
    mods: KeyModifiers,
    serial_monitor: Option<&mut SerialMonitorState>,
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    match (code, mods) {
        (KeyCode::Esc, _) => Action::GoSerial,
        (KeyCode::Up, _) | (KeyCode::BackTab, _) => {
            state.usart_isp.field = state.usart_isp.field.prev();
            Action::None
        }
        (KeyCode::Down, _) | (KeyCode::Tab, KeyModifiers::NONE) => {
            state.usart_isp.field = state.usart_isp.field.next();
            Action::None
        }
        (KeyCode::Left, _) if state.usart_isp.field == IspConfigField::FilePath => {
            state.usart_isp.file_cursor_left();
            Action::None
        }
        (KeyCode::Right, _) if state.usart_isp.field == IspConfigField::FilePath => {
            state.usart_isp.file_cursor_right();
            Action::None
        }
        (KeyCode::Home, _) if state.usart_isp.field == IspConfigField::FilePath => {
            state.usart_isp.file_cursor_home();
            Action::None
        }
        (KeyCode::End, _) if state.usart_isp.field == IspConfigField::FilePath => {
            state.usart_isp.file_cursor_end();
            Action::None
        }
        (KeyCode::Left, _) => {
            cycle_option(state, false);
            Action::None
        }
        (KeyCode::Right, _) => {
            cycle_option(state, true);
            Action::None
        }
        (KeyCode::Backspace, _) if state.usart_isp.field == IspConfigField::FilePath => {
            state.usart_isp.file_backspace();
            Action::None
        }
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT)
            if state.usart_isp.field == IspConfigField::FilePath =>
        {
            state.usart_isp.file_input_char(c);
            Action::None
        }
        (KeyCode::Enter, _) => start_flash(state, serial_monitor, tx),
        _ => Action::None,
    }
}

fn start_flash(
    state: &mut FlasherState,
    serial_monitor: Option<&mut SerialMonitorState>,
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    let mut preflight_log = None;
    state.clear_serial_monitor_restore();
    if let Some(port_name) = selected_port_name(state).map(str::to_string) {
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
    if let Err(e) = isp::start_flash(state, tx) {
        state.log.push(format!("Error: {}", e));
        state.op_done = true;
        state.op_ok = false;
    }
    Action::None
}

fn selected_port_name(state: &FlasherState) -> Option<&str> {
    state
        .usart_isp
        .port_list
        .get(state.usart_isp.port_idx)
        .map(String::as_str)
        .filter(|name| *name != "(no ports found)")
}

fn cycle_option(state: &mut FlasherState, forward: bool) {
    match state.usart_isp.field {
        IspConfigField::Port => {
            if state.usart_isp.port_list.is_empty() {
                return;
            }
            if forward {
                state.usart_isp.port_idx =
                    (state.usart_isp.port_idx + 1) % state.usart_isp.port_list.len();
            } else if state.usart_isp.port_idx == 0 {
                state.usart_isp.port_idx = state.usart_isp.port_list.len() - 1;
            } else {
                state.usart_isp.port_idx -= 1;
            }
        }
        IspConfigField::BaudRate => {
            if forward {
                state.usart_isp.baud_idx = (state.usart_isp.baud_idx + 1)
                    % crate::transport::serial::ISP_BAUD_PRESETS.len();
            } else if state.usart_isp.baud_idx == 0 {
                state.usart_isp.baud_idx = crate::transport::serial::ISP_BAUD_PRESETS.len() - 1;
            } else {
                state.usart_isp.baud_idx -= 1;
            }
        }
        IspConfigField::BootMode => {
            state.usart_isp.boot_mode = if forward {
                state.usart_isp.boot_mode.next()
            } else {
                state.usart_isp.boot_mode.prev()
            };
        }
        IspConfigField::FilePath => {}
    }
}
