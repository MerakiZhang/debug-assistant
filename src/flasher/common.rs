use std::path::Path;
use std::sync::mpsc;

use anyhow::bail;
use probe_rs::flashing;
use probe_rs::probe::list::Lister;

use crate::event::AppEvent;

pub fn detect_file_kind(file_path: &str) -> Option<flashing::FormatKind> {
    let lower = file_path.to_ascii_lowercase();
    if lower.ends_with(".bin") {
        Some(flashing::FormatKind::Bin)
    } else if lower.ends_with(".hex") {
        Some(flashing::FormatKind::Hex)
    } else {
        None
    }
}

pub fn validate_firmware_path(file_path: &str) -> anyhow::Result<()> {
    if file_path.is_empty() {
        bail!("File path is empty");
    }
    if detect_file_kind(file_path).is_none() {
        bail!("Only .bin/.hex files are supported (got: {})", file_path);
    }
    if !Path::new(file_path).exists() {
        bail!("File not found: {}", file_path);
    }
    Ok(())
}

pub fn list_probes() -> Vec<String> {
    let probes = Lister::new().list_all();
    if probes.is_empty() {
        return vec!["(no probes found)".to_string()];
    }

    probes
        .iter()
        .enumerate()
        .map(|(idx, probe)| format!("{}: {}", idx, probe))
        .collect()
}

pub fn send_done(tx: &mpsc::Sender<AppEvent>, success: bool, message: String) {
    tx.send(AppEvent::FlasherDone { success, message }).ok();
}
