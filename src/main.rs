mod event;
mod flasher;
mod home;
mod root_app;
mod root_ui;
mod serial;
mod serial_monitor;

use event::AppEvent;
use root_app::RootApp;
use std::sync::mpsc;

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<AppEvent>();
    let mut app = RootApp::new(tx.clone());

    event::spawn_event_thread(tx.clone());

    loop {
        terminal.draw(|frame| root_ui::render(frame, &app))?;

        match rx.recv()? {
            AppEvent::Tick => app.on_tick(),

            AppEvent::Key(code, mods) => app.on_key(code, mods)?,

            AppEvent::Resize => {}

            AppEvent::SerialData(bytes) => {
                app.serial_monitor.bytes_rx += bytes.len() as u64;
                app.serial_monitor.push_rx(bytes);
            }
            AppEvent::SerialError(msg) => {
                let was = app.serial_monitor.connected;
                app.serial_monitor.disconnect_with_status(false);
                if was {
                    app.serial_monitor
                        .push_status(format!("Serial error: {}", msg));
                }
            }
            AppEvent::SerialDisconnected => {
                app.serial_monitor.disconnect();
            }

            AppEvent::FlasherLog(msg) => {
                app.flasher.log.push(msg);
            }
            AppEvent::FlasherProgress(pct) => {
                app.flasher.progress_pct = Some(pct);
            }
            AppEvent::FlasherDone { success, message } => {
                app.flasher.op_done = true;
                app.flasher.op_ok = success;
                app.flasher.log.push(message);
                app.flasher.progress_pct = if success {
                    Some(100)
                } else {
                    app.flasher.progress_pct
                };
                app.flasher.stop_flag = None;
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
