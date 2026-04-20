mod app;
mod event;
mod serial;
mod ui;

use std::sync::mpsc;
use crossterm::event::{KeyCode, KeyModifiers};
use app::{App, Focus};
use event::AppEvent;

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<AppEvent>();
    let mut app = App::new();

    event::spawn_event_thread(tx.clone());

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        match rx.recv()? {
            AppEvent::Tick => {
                app.flush_rx_buf();
                app.ensure_auto_scroll();
            }
            AppEvent::Key(code, mods) => {
                handle_key(&mut app, code, mods, tx.clone())?;
            }
            AppEvent::Resize => {}
            AppEvent::SerialData(bytes) => {
                app.bytes_rx += bytes.len() as u64;
                app.push_rx(bytes);
            }
            AppEvent::SerialError(msg) => {
                let was = app.connected;
                app.disconnect();
                if was {
                    app.push_status(format!("Serial error: {}", msg));
                }
            }
            AppEvent::SerialDisconnected => {
                app.disconnect();
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_key(
    app: &mut App,
    code: KeyCode,
    mods: KeyModifiers,
    tx: mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    // Help overlay: any key closes it
    if app.show_help {
        app.show_help = false;
        app.focus = app.prev_focus;
        return Ok(());
    }

    // Global shortcuts (always active)
    match (code, mods) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return Ok(());
        }
        // 'q' quits only when not typing in the send panel
        (KeyCode::Char('q'), KeyModifiers::NONE) if app.focus != Focus::Send => {
            app.should_quit = true;
            return Ok(());
        }
        (KeyCode::F(1), _) => {
            app.prev_focus = app.focus;
            app.show_help = true;
            app.focus = Focus::HelpOverlay;
            return Ok(());
        }
        (KeyCode::F(2), _) if !app.show_config => {
            app.open_config_popup();
            return Ok(());
        }
        (KeyCode::F(3), _) => {
            app.clear_log();
            return Ok(());
        }
        (KeyCode::F(4), _) => {
            app.display_mode = app.display_mode.next();
            return Ok(());
        }
        (KeyCode::F(5), _) => {
            app.auto_scroll = !app.auto_scroll;
            if app.auto_scroll {
                app.ensure_auto_scroll();
            }
            return Ok(());
        }
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            app.disconnect();
            return Ok(());
        }
        (KeyCode::Tab, KeyModifiers::NONE) if !app.show_config => {
            app.focus = match app.focus {
                Focus::Receive => Focus::Send,
                Focus::Send    => Focus::Receive,
                other          => other,
            };
            return Ok(());
        }
        _ => {}
    }

    match app.focus {
        Focus::ConfigPopup  => handle_config_key(app, code, mods, tx)?,
        Focus::Send         => handle_send_key(app, code, mods)?,
        Focus::Receive      => handle_receive_key(app, code, mods),
        Focus::HelpOverlay  => {}
    }
    Ok(())
}

fn handle_config_key(
    app: &mut App,
    code: KeyCode,
    _mods: KeyModifiers,
    tx: mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    match code {
        KeyCode::Esc => app.cancel_config(),
        KeyCode::Enter => {
            if let Err(e) = app.apply_config_and_close(tx) {
                app.show_config = false;
                app.focus = Focus::Send;
                app.push_status(format!("Connect failed: {}", e));
            }
        }
        KeyCode::Up | KeyCode::BackTab => app.config_field = app.config_field.prev(),
        KeyCode::Down | KeyCode::Tab   => app.config_field = app.config_field.next(),
        KeyCode::Left  => app.config_field_prev_option(),
        KeyCode::Right => app.config_field_next_option(),
        _ => {}
    }
    Ok(())
}

fn handle_send_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> anyhow::Result<()> {
    match (code, mods) {
        (KeyCode::Enter, _) => {
            if let Some(bytes) = app.send_current_input() {
                if let Some(ref serial_tx) = app.serial_tx {
                    let _ = serial_tx.send(bytes);
                }
            }
        }
        (KeyCode::Up, _)    => app.history_up(),
        (KeyCode::Down, _)  => app.history_down(),
        (KeyCode::Left, _)  => app.input_cursor_left(),
        (KeyCode::Right, _) => app.input_cursor_right(),
        (KeyCode::Home, _)  => app.input_cursor_home(),
        (KeyCode::End, _)   => app.input_cursor_end(),
        (KeyCode::Backspace, _) => app.input_backspace(),
        (KeyCode::Delete, _)    => app.input_delete(),
        (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
            app.hex_send_mode = !app.hex_send_mode;
        }
        (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            app.newline_suffix = app.newline_suffix.cycle();
        }
        (KeyCode::Char(c), KeyModifiers::NONE) |
        (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            app.input_char(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_receive_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) {
    let visible = app.rx_visible_rows.get() as usize;
    match code {
        KeyCode::Up       => app.scroll_up(1),
        KeyCode::Down     => app.scroll_down(1),
        KeyCode::PageUp   => app.scroll_up(visible.saturating_sub(1)),
        KeyCode::PageDown => app.scroll_down(visible.saturating_sub(1)),
        KeyCode::Home     => {
            app.log_scroll = 0;
            app.auto_scroll = false;
        }
        KeyCode::End => {
            app.auto_scroll = true;
            app.ensure_auto_scroll();
        }
        _ => {}
    }
}
