use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::sync::mpsc::Sender;
use std::time::Duration;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyCode, KeyModifiers),
    Resize,
    Tick,
    SerialData(Vec<u8>),
    SerialError(String),
    FlasherLog(String),
    FlasherProgress(u8),
    FlasherDone { success: bool, message: String },
}

pub fn spawn_event_thread(tx: Sender<AppEvent>) {
    std::thread::spawn(move || loop {
        if event::poll(Duration::from_millis(50)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => {
                    if tx.send(AppEvent::Key(k.code, k.modifiers)).is_err() {
                        break;
                    }
                }
                Ok(Event::Resize(_, _)) => {
                    if tx.send(AppEvent::Resize).is_err() {
                        break;
                    }
                }
                _ => {}
            }
        } else if tx.send(AppEvent::Tick).is_err() {
            break;
        }
    });
}
