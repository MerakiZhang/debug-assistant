pub mod controller;
pub mod input;
pub mod log;
pub mod service;
pub mod state;
pub mod ui;

pub use state::SerialMonitorState;

use crate::app::event::AppEvent;
use crossterm::event::{KeyCode, KeyModifiers};
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
    controller::handle_key(state, code, mods, tx)
}
