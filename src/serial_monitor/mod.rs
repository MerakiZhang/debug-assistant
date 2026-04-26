pub mod state;
pub mod ui;

pub use state::SerialMonitorState;

use crate::event::AppEvent;
use crate::log_export;
use crossterm::event::{KeyCode, KeyModifiers};
use state::Focus;
use std::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
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
        (KeyCode::F(1), _) => {
            state.prev_focus = state.focus;
            state.show_help = true;
            state.focus = Focus::HelpOverlay;
            return Ok(Action::None);
        }
        (KeyCode::F(2), _) => {
            state.focus_setup();
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
        (KeyCode::F(6), _) => {
            copy_log_to_clipboard(state);
            return Ok(Action::None);
        }
        (KeyCode::F(7), _) => {
            save_log_to_file(state);
            return Ok(Action::None);
        }
        // Tab cycles focus: Setup → Receive → Send → Setup
        (KeyCode::Tab, KeyModifiers::NONE) => {
            state.focus = match state.focus {
                Focus::Setup => Focus::Receive,
                Focus::Receive => Focus::Send,
                Focus::Send => {
                    state.focus_setup();
                    return Ok(Action::None);
                }
                Focus::HelpOverlay => Focus::Send,
            };
            return Ok(Action::None);
        }
        (KeyCode::BackTab, _) => {
            state.focus = match state.focus {
                Focus::Setup => Focus::Send,
                Focus::Receive => {
                    state.focus_setup();
                    return Ok(Action::None);
                }
                Focus::Send => Focus::Receive,
                Focus::HelpOverlay => Focus::Send,
            };
            return Ok(Action::None);
        }
        _ => {}
    }

    match state.focus {
        Focus::Setup => handle_setup_key(state, code, mods, tx)?,
        Focus::Send => handle_send_key(state, code, mods)?,
        Focus::Receive => handle_receive_key(state, code, mods),
        Focus::HelpOverlay => {}
    }
    Ok(Action::None)
}

fn copy_log_to_clipboard(state: &mut SerialMonitorState) {
    let text = state.export_log_text();
    match log_export::copy_to_clipboard(&text) {
        Ok(()) => state.push_status("Log copied to clipboard".to_string()),
        Err(e) => state.push_status(format!("Copy log failed: {}", e)),
    }
}

fn save_log_to_file(state: &mut SerialMonitorState) {
    let text = state.export_log_text();
    match log_export::save_log("serial", &text) {
        Ok(path) => state.push_status(format!("Log saved to {}", path.display())),
        Err(e) => state.push_status(format!("Save log failed: {}", e)),
    }
}

fn handle_setup_key(
    state: &mut SerialMonitorState,
    code: KeyCode,
    mods: KeyModifiers,
    tx: mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    use state::ConfigField;
    match (code, mods) {
        (KeyCode::Esc, _) => state.cancel_setup(),
        (KeyCode::Enter, _) => {
            if let Err(e) = state.apply_setup(tx) {
                state.focus = Focus::Send;
                state.push_status(format!("Connect failed: {}", e));
            }
        }
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            state.disconnect();
            state.focus = Focus::Send;
        }
        (KeyCode::Char('r'), KeyModifiers::NONE)
            if state.config_field == ConfigField::PortName =>
        {
            state.refresh_port_list();
        }
        (KeyCode::Up, _) => state.config_field = state.config_field.prev(),
        (KeyCode::Down, _) => state.config_field = state.config_field.next(),
        (KeyCode::Left, _) => state.config_field_prev_option(),
        (KeyCode::Right, _) => state.config_field_next_option(),
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
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            state.disconnect();
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
