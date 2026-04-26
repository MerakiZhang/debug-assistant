use std::sync::mpsc;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::event::AppEvent;
use crate::features::protocols::flashing::state::{
    SwdConfigField, SWD_CHIP_PRESETS, SWD_SPEED_PRESETS,
};
use crate::features::protocols::flashing::{Action, FlasherState};
use crate::features::protocols::swd::flash;

pub fn handle_config(
    state: &mut FlasherState,
    code: KeyCode,
    mods: KeyModifiers,
    tx: mpsc::Sender<AppEvent>,
) -> Action {
    match (code, mods) {
        (KeyCode::Esc, _) => Action::GoHome,
        (KeyCode::Char('r'), KeyModifiers::NONE) => handle_r_key(state),
        (KeyCode::Up, _) | (KeyCode::BackTab, _) => {
            state.swd.field = state.swd.field.prev();
            Action::None
        }
        (KeyCode::Down, _) | (KeyCode::Tab, KeyModifiers::NONE) => {
            state.swd.field = state.swd.field.next();
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
        (KeyCode::Backspace, _) => {
            handle_backspace(state);
            Action::None
        }
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            input_char(state, c);
            Action::None
        }
        (KeyCode::Enter, _) => {
            state.enter_progress();
            if let Err(e) = flash::start_flash(state, tx) {
                state.log.push(format!("Error: {}", e));
                state.op_done = true;
                state.op_ok = false;
            }
            Action::None
        }
        _ => Action::None,
    }
}

fn handle_r_key(state: &mut FlasherState) -> Action {
    if state.swd.field == SwdConfigField::Probe {
        state.refresh_swd_probes();
    } else {
        match state.swd.field {
            SwdConfigField::BinBaseAddress => state.swd.bin_base_input_char('r'),
            SwdConfigField::ChipName => state.swd.chip_input_char('r'),
            SwdConfigField::FilePath => state.swd.file_input_char('r'),
            _ => {}
        }
    }
    Action::None
}

fn handle_backspace(state: &mut FlasherState) {
    match state.swd.field {
        SwdConfigField::BinBaseAddress => state.swd.bin_base_backspace(),
        SwdConfigField::ChipName => state.swd.chip_backspace(),
        SwdConfigField::FilePath => state.swd.file_backspace(),
        SwdConfigField::ChipPreset => {}
        _ => {}
    }
}

fn input_char(state: &mut FlasherState, c: char) {
    match state.swd.field {
        SwdConfigField::BinBaseAddress => state.swd.bin_base_input_char(c),
        SwdConfigField::ChipName => state.swd.chip_input_char(c),
        SwdConfigField::FilePath => state.swd.file_input_char(c),
        SwdConfigField::ChipPreset => {}
        _ => {}
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

fn cycle_option(state: &mut FlasherState, forward: bool) {
    match state.swd.field {
        SwdConfigField::Probe => cycle_probe_idx(
            &mut state.swd.probe_idx,
            state.swd.probe_list.len(),
            forward,
        ),
        SwdConfigField::Speed => {
            if forward {
                state.swd.speed_idx = (state.swd.speed_idx + 1) % SWD_SPEED_PRESETS.len();
            } else if state.swd.speed_idx == 0 {
                state.swd.speed_idx = SWD_SPEED_PRESETS.len() - 1;
            } else {
                state.swd.speed_idx -= 1;
            }
        }
        SwdConfigField::ConnectMode => {
            state.swd.connect_mode = if forward {
                state.swd.connect_mode.next()
            } else {
                state.swd.connect_mode.prev()
            };
        }
        SwdConfigField::Verify => state.swd.verify = !state.swd.verify,
        SwdConfigField::ResetRun => state.swd.reset_run = !state.swd.reset_run,
        SwdConfigField::ChipPreset => {
            if forward {
                state.swd.chip_preset_idx =
                    (state.swd.chip_preset_idx + 1) % SWD_CHIP_PRESETS.len();
            } else if state.swd.chip_preset_idx == 0 {
                state.swd.chip_preset_idx = SWD_CHIP_PRESETS.len() - 1;
            } else {
                state.swd.chip_preset_idx -= 1;
            }
            state.swd_apply_chip_preset();
        }
        SwdConfigField::BinBaseAddress | SwdConfigField::ChipName | SwdConfigField::FilePath => {}
    }
}
