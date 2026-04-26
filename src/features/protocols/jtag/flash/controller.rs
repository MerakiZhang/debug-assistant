use std::sync::mpsc;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::event::AppEvent;
use crate::features::protocols::flashing::state::{
    JtagConfigField, JTAG_CHIP_PRESETS, JTAG_SPEED_PRESETS,
};
use crate::features::protocols::flashing::{Action, FlasherState};
use crate::features::protocols::jtag::flash;

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
            state.jtag.field = state.jtag.field.prev();
            Action::None
        }
        (KeyCode::Down, _) | (KeyCode::Tab, KeyModifiers::NONE) => {
            state.jtag.field = state.jtag.field.next();
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
    if state.jtag.field == JtagConfigField::Probe {
        state.refresh_jtag_probes();
    } else {
        match state.jtag.field {
            JtagConfigField::BinBaseAddress => state.jtag.bin_base_input_char('r'),
            JtagConfigField::ChipName => state.jtag.chip_input_char('r'),
            JtagConfigField::FilePath => state.jtag.file_input_char('r'),
            JtagConfigField::ChipPreset => {}
            _ => {}
        }
    }
    Action::None
}

fn handle_backspace(state: &mut FlasherState) {
    match state.jtag.field {
        JtagConfigField::BinBaseAddress => state.jtag.bin_base_backspace(),
        JtagConfigField::ChipName => state.jtag.chip_backspace(),
        JtagConfigField::FilePath => state.jtag.file_backspace(),
        JtagConfigField::ChipPreset => {}
        _ => {}
    }
}

fn input_char(state: &mut FlasherState, c: char) {
    match state.jtag.field {
        JtagConfigField::BinBaseAddress => state.jtag.bin_base_input_char(c),
        JtagConfigField::ChipName => state.jtag.chip_input_char(c),
        JtagConfigField::FilePath => state.jtag.file_input_char(c),
        JtagConfigField::ChipPreset => {}
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
    match state.jtag.field {
        JtagConfigField::Probe => cycle_probe_idx(
            &mut state.jtag.probe_idx,
            state.jtag.probe_list.len(),
            forward,
        ),
        JtagConfigField::Speed => {
            if forward {
                state.jtag.speed_idx = (state.jtag.speed_idx + 1) % JTAG_SPEED_PRESETS.len();
            } else if state.jtag.speed_idx == 0 {
                state.jtag.speed_idx = JTAG_SPEED_PRESETS.len() - 1;
            } else {
                state.jtag.speed_idx -= 1;
            }
        }
        JtagConfigField::Verify => state.jtag.verify = !state.jtag.verify,
        JtagConfigField::ResetRun => state.jtag.reset_run = !state.jtag.reset_run,
        JtagConfigField::ChipPreset => {
            if forward {
                state.jtag.chip_preset_idx =
                    (state.jtag.chip_preset_idx + 1) % JTAG_CHIP_PRESETS.len();
            } else if state.jtag.chip_preset_idx == 0 {
                state.jtag.chip_preset_idx = JTAG_CHIP_PRESETS.len() - 1;
            } else {
                state.jtag.chip_preset_idx -= 1;
            }
            state.jtag_apply_chip_preset();
        }
        JtagConfigField::BinBaseAddress | JtagConfigField::ChipName | JtagConfigField::FilePath => {
        }
    }
}
