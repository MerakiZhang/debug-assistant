use crate::flasher;
use crate::home;
use crate::root_app::{RootApp, Screen};
use crate::serial_monitor;
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &RootApp) {
    match app.current_screen {
        Screen::Home => home::render(frame, &app.home),
        Screen::SerialMonitor => serial_monitor::render(frame, &app.serial_monitor),
        Screen::Flasher => flasher::render(frame, &app.flasher),
    }
}
