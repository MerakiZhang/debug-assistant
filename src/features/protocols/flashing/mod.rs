pub mod common;
pub mod controller;
pub mod state;
pub mod ui;

pub use state::FlasherState;
use state::{FlasherMethod, FlasherSubScreen};

use crate::app::event::AppEvent;
use crate::core::log as log_export;
use crate::features::protocols::uart::monitor::SerialMonitorState;
use crate::features::protocols::{jtag, swd, uart};
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    GoHome,
    GoSerial,
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
    // F6/F7 work on any sub-screen
    match code {
        KeyCode::F(6) => {
            let text = state.log.join("\n");
            match log_export::copy_to_clipboard(&text) {
                Ok(()) => state.log.push("Log copied to clipboard.".into()),
                Err(e) => state.log.push(format!("Copy log failed: {}", e)),
            }
            state.scroll_log_end();
            return Action::None;
        }
        KeyCode::F(7) => {
            let text = state.log.join("\n");
            match log_export::save_log("flasher", &text) {
                Ok(path) => state.log.push(format!("Log saved to {}", path.display())),
                Err(e) => state.log.push(format!("Save log failed: {}", e)),
            }
            state.scroll_log_end();
            return Action::None;
        }
        _ => {}
    }

    match state.sub_screen {
        FlasherSubScreen::Config => handle_config(state, code, mods, serial_monitor, tx),
        FlasherSubScreen::Progress => controller::handle_progress(state, code),
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
        FlasherMethod::UsartIsp => {
            uart::isp::controller::handle_config(state, code, mods, serial_monitor, tx)
        }
        FlasherMethod::Jtag => jtag::flash::controller::handle_config(state, code, mods, tx),
        FlasherMethod::Swd => swd::flash::controller::handle_config(state, code, mods, tx),
    }
}
