use crate::event::AppEvent;
use crate::flasher::{self, FlasherState};
use crate::home::{self, HomeState};
use crate::serial_monitor::{self, SerialMonitorState};
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Serial,
    SerialMonitor,
    Flasher,
}

pub struct RootApp {
    pub current_screen: Screen,
    pub should_quit: bool,
    pub event_tx: mpsc::Sender<AppEvent>,
    pub home: HomeState,
    pub serial_selected: usize,
    pub serial_monitor: SerialMonitorState,
    pub flasher: FlasherState,
}

impl RootApp {
    pub fn new(event_tx: mpsc::Sender<AppEvent>) -> Self {
        Self {
            current_screen: Screen::Home,
            should_quit: false,
            event_tx,
            home: HomeState::new(),
            serial_selected: 0,
            serial_monitor: SerialMonitorState::new(),
            flasher: FlasherState::new(),
        }
    }

    pub fn on_tick(&mut self) {
        self.serial_monitor.flush_rx_buf();
        self.serial_monitor.ensure_auto_scroll();
    }

    pub fn on_key(&mut self, code: KeyCode, mods: KeyModifiers) -> anyhow::Result<()> {
        // Global: Ctrl+C always quits from any screen
        if code == KeyCode::Char('c') && mods == KeyModifiers::CONTROL {
            self.flasher.request_stop();
            self.should_quit = true;
            return Ok(());
        }

        match self.current_screen {
            Screen::Home => {
                if let Some(action) = home::handle_key(&mut self.home, code, mods) {
                    match action {
                        home::HomeAction::Navigate(screen) => self.current_screen = screen,
                        home::HomeAction::OpenFlasher(method) => {
                            self.flasher.enter_protocol_config(method);
                            self.current_screen = Screen::Flasher;
                        }
                        home::HomeAction::Quit => self.should_quit = true,
                    }
                }
            }

            Screen::Serial => match code {
                KeyCode::Esc => self.current_screen = Screen::Home,
                KeyCode::Char('q') if mods == KeyModifiers::NONE => self.should_quit = true,
                KeyCode::Up => {
                    self.serial_selected = if self.serial_selected == 0 { 1 } else { 0 };
                }
                KeyCode::Down => {
                    self.serial_selected = (self.serial_selected + 1) % 2;
                }
                KeyCode::Enter => {
                    if self.serial_selected == 0 {
                        self.current_screen = Screen::SerialMonitor;
                    } else {
                        self.flasher
                            .enter_protocol_config(flasher::state::FlasherMethod::UsartIsp);
                        self.current_screen = Screen::Flasher;
                    }
                }
                _ => {}
            },

            Screen::SerialMonitor => {
                // Esc returns to the Serial protocol page when not in Setup or help overlay.
                if code == KeyCode::Esc
                    && mods == KeyModifiers::NONE
                    && self.serial_monitor.focus != serial_monitor::state::Focus::Setup
                    && !self.serial_monitor.show_help
                {
                    self.serial_monitor.disconnect();
                    self.current_screen = Screen::Serial;
                    return Ok(());
                }
                let _action = serial_monitor::handle_key(
                    &mut self.serial_monitor,
                    code,
                    mods,
                    self.event_tx.clone(),
                )?;
            }

            Screen::Flasher => {
                let action = flasher::handle_key(
                    &mut self.flasher,
                    code,
                    mods,
                    Some(&mut self.serial_monitor),
                    self.event_tx.clone(),
                );
                match action {
                    flasher::Action::GoHome => self.current_screen = Screen::Home,
                    flasher::Action::GoSerial => self.current_screen = Screen::Serial,
                    flasher::Action::None => {}
                }
            }
        }
        Ok(())
    }
}
