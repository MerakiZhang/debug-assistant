pub mod state;
pub mod ui;

pub use state::SerialMonitorState;

use crate::event::AppEvent;
use crossterm::event::{KeyCode, KeyModifiers};
use state::Focus;
use std::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
}

pub fn render(frame: &mut ratatui::Frame, state: &SerialMonitorState) {
    ui::render(frame, state);
}

pub fn handle_key(
    state: &mut SerialMonitorState,
    code: KeyCode,
    mods: KeyModifiers,
    tx: mpsc::Sender<AppEvent>,
) -> anyhow::Result<Action> {
    // Help overlay: any key closes it
    if state.show_help {
        state.show_help = false;
        state.focus = state.prev_focus;
        return Ok(Action::None);
    }

    // Global shortcuts
    match (code, mods) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => {
            return Ok(Action::Quit);
        }
        (KeyCode::F(1), _) => {
            state.prev_focus = state.focus;
            state.show_help = true;
            state.focus = Focus::HelpOverlay;
            return Ok(Action::None);
        }
        (KeyCode::F(2), _) if !state.show_config => {
            state.open_config_popup();
            return Ok(Action::None);
        }
        (KeyCode::F(3), _) => {
            state.clear_log();
            return Ok(Action::None);
        }
        (KeyCode::F(4), _) => {
            state.display_mode = state.display_mode.next();
            return Ok(Action::None);
        }
        (KeyCode::F(5), _) => {
            state.auto_scroll = !state.auto_scroll;
            if state.auto_scroll {
                state.ensure_auto_scroll();
            }
            return Ok(Action::None);
        }
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            state.disconnect();
            return Ok(Action::None);
        }
        (KeyCode::Tab, KeyModifiers::NONE) if !state.show_config => {
            state.focus = match state.focus {
                Focus::Receive => Focus::Send,
                Focus::Send => Focus::Receive,
                other => other,
            };
            return Ok(Action::None);
        }
        _ => {}
    }

    match state.focus {
        Focus::ConfigPopup => handle_config_key(state, code, mods, tx)?,
        Focus::Send => handle_send_key(state, code, mods)?,
        Focus::Receive => handle_receive_key(state, code, mods),
        Focus::HelpOverlay => {}
    }
    Ok(Action::None)
}

fn handle_config_key(
    state: &mut SerialMonitorState,
    code: KeyCode,
    _mods: KeyModifiers,
    tx: mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    match code {
        KeyCode::Esc => state.cancel_config(),
        KeyCode::Enter => {
            if let Err(e) = state.apply_config_and_close(tx) {
                state.show_config = false;
                state.focus = Focus::Send;
                state.push_status(format!("Connect failed: {}", e));
            }
        }
        KeyCode::Up | KeyCode::BackTab => state.config_field = state.config_field.prev(),
        KeyCode::Down | KeyCode::Tab => state.config_field = state.config_field.next(),
        KeyCode::Left => state.config_field_prev_option(),
        KeyCode::Right => state.config_field_next_option(),
        _ => {}
    }
    Ok(())
}

fn handle_send_key(
    state: &mut SerialMonitorState,
    code: KeyCode,
    mods: KeyModifiers,
) -> anyhow::Result<()> {
    match (code, mods) {
        (KeyCode::Enter, _) => {
            if state.input_buf.is_empty() {
                return Ok(());
            }
            let Some(serial_tx) = state.serial_tx.clone() else {
                state.push_status("Not connected".to_string());
                return Ok(());
            };
            if let Some(bytes) = state.prepare_current_input_bytes() {
                match serial_tx.send(bytes.clone()) {
                    Ok(()) => state.commit_sent_input(bytes),
                    Err(_) => state.push_status("Send failed: disconnected".to_string()),
                }
            }
        }
        (KeyCode::Up, _) => state.history_up(),
        (KeyCode::Down, _) => state.history_down(),
        (KeyCode::Left, _) => state.input_cursor_left(),
        (KeyCode::Right, _) => state.input_cursor_right(),
        (KeyCode::Home, _) => state.input_cursor_home(),
        (KeyCode::End, _) => state.input_cursor_end(),
        (KeyCode::Backspace, _) => state.input_backspace(),
        (KeyCode::Delete, _) => state.input_delete(),
        (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
            state.hex_send_mode = !state.hex_send_mode;
        }
        (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            state.newline_suffix = state.newline_suffix.cycle();
        }
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            state.input_char(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_receive_key(state: &mut SerialMonitorState, code: KeyCode, _mods: KeyModifiers) {
    let visible = state.rx_visible_rows.get() as usize;
    match code {
        KeyCode::Up => state.scroll_up(1),
        KeyCode::Down => state.scroll_down(1),
        KeyCode::PageUp => state.scroll_up(visible.saturating_sub(1)),
        KeyCode::PageDown => state.scroll_down(visible.saturating_sub(1)),
        KeyCode::Home => {
            state.log_scroll = 0;
            state.auto_scroll = false;
        }
        KeyCode::End => {
            state.auto_scroll = true;
            state.ensure_auto_scroll();
        }
        _ => {}
    }
}
