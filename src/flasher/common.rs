use std::path::Path;
use std::sync::mpsc;

use anyhow::{bail, Context};
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

pub fn parse_address(input: &str) -> anyhow::Result<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("BIN base address is empty");
    }

    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).context("BIN base address must be a valid hex number")
    } else {
        trimmed
            .parse::<u64>()
            .context("BIN base address must be a valid decimal or hex number")
    }
}

pub fn format_for_download(
    file_path: &str,
    base_address: Option<u64>,
) -> anyhow::Result<flashing::Format> {
    match detect_file_kind(file_path) {
        Some(flashing::FormatKind::Bin) => Ok(flashing::Format::Bin(flashing::BinOptions {
            base_address,
            skip: 0,
        })),
        Some(flashing::FormatKind::Hex) => Ok(flashing::Format::Hex),
        _ => bail!("Only .bin/.hex files are supported (got: {})", file_path),
    }
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
