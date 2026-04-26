use crate::app::event::AppEvent;
use crate::features::home::{self, HomeState};
use crate::features::protocols::flashing::{self as flasher, FlasherState};
use crate::features::protocols::uart::monitor::{self, SerialMonitorState};
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Uart,
    Jtag,
    Swd,
    I2c,
    Spi,
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
                        home::HomeAction::Quit => self.should_quit = true,
                    }
                }
            }

            Screen::Uart => match code {
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

            Screen::Jtag => match code {
                KeyCode::Esc => self.current_screen = Screen::Home,
                KeyCode::Char('q') if mods == KeyModifiers::NONE => self.should_quit = true,
                KeyCode::Enter => {
                    self.flasher
                        .enter_protocol_config(flasher::state::FlasherMethod::Jtag);
                    self.current_screen = Screen::Flasher;
                }
                _ => {}
            },

            Screen::Swd => match code {
                KeyCode::Esc => self.current_screen = Screen::Home,
                KeyCode::Char('q') if mods == KeyModifiers::NONE => self.should_quit = true,
                KeyCode::Enter => {
                    self.flasher
                        .enter_protocol_config(flasher::state::FlasherMethod::Swd);
                    self.current_screen = Screen::Flasher;
                }
                _ => {}
            },

            Screen::I2c | Screen::Spi => match code {
                KeyCode::Esc => self.current_screen = Screen::Home,
                KeyCode::Char('q') if mods == KeyModifiers::NONE => self.should_quit = true,
                _ => {}
            },

            Screen::SerialMonitor => {
                // Esc returns to the Serial protocol page when not in Setup or help overlay.
                if code == KeyCode::Esc
                    && mods == KeyModifiers::NONE
                    && self.serial_monitor.focus != monitor::state::Focus::Setup
                    && !self.serial_monitor.show_help
                {
                    self.serial_monitor.disconnect();
                    self.current_screen = Screen::Uart;
                    return Ok(());
                }
                let _action = monitor::handle_key(
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
                    flasher::Action::GoHome => {
                        self.current_screen = match self.flasher.method {
                            flasher::state::FlasherMethod::UsartIsp => Screen::Uart,
                            flasher::state::FlasherMethod::Jtag => Screen::Jtag,
                            flasher::state::FlasherMethod::Swd => Screen::Swd,
                        }
                    }
                    flasher::Action::GoSerial => self.current_screen = Screen::Uart,
                    flasher::Action::None => {}
                }
            }
        }
        Ok(())
    }
}
