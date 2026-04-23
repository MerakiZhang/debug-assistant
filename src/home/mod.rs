mod state;
mod ui;

pub use state::HomeState;
use state::MENU_ITEMS;

use crate::root_app::Screen;
use crossterm::event::{KeyCode, KeyModifiers};

pub enum HomeAction {
    Navigate(Screen),
    Quit,
}

pub fn render(frame: &mut ratatui::Frame, state: &HomeState) {
    ui::render(frame, state);
}

pub fn handle_key(state: &mut HomeState, code: KeyCode, _mods: KeyModifiers) -> Option<HomeAction> {
    match code {
        KeyCode::Up => {
            if state.selected == 0 {
                state.selected = MENU_ITEMS.len() - 1;
            } else {
                state.selected -= 1;
            }
        }
        KeyCode::Down => {
            state.selected = (state.selected + 1) % MENU_ITEMS.len();
        }
        KeyCode::Enter => {
            return Some(match state.selected {
                0 => HomeAction::Navigate(Screen::SerialMonitor),
                1 => HomeAction::Navigate(Screen::Flasher),
                _ => HomeAction::Quit,
            });
        }
        KeyCode::Char('q') => return Some(HomeAction::Quit),
        _ => {}
    }
    None
}
