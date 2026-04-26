use super::state::{FlasherState, FlasherSubScreen};
use super::Action;
use crossterm::event::KeyCode;

pub fn handle_progress(state: &mut FlasherState, code: KeyCode) -> Action {
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

            state.request_stop();
            state.sub_screen = FlasherSubScreen::Config;
            Action::None
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
