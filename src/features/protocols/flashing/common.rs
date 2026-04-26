use std::sync::mpsc;

use crate::app::event::AppEvent;

pub fn send_done(tx: &mpsc::Sender<AppEvent>, success: bool, message: String) {
    tx.send(AppEvent::FlasherDone { success, message }).ok();
}
