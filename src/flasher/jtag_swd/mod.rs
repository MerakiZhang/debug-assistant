use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use anyhow::{bail, Context};
use probe_rs::flashing;
use probe_rs::probe::list::Lister;
use probe_rs::Permissions;

use crate::event::AppEvent;
use crate::flasher::state::FlasherState;

fn detect_file_kind(file_path: &str) -> Option<flashing::FormatKind> {
    let lower = file_path.to_ascii_lowercase();
    if lower.ends_with(".bin") {
        Some(flashing::FormatKind::Bin)
    } else if lower.ends_with(".hex") {
        Some(flashing::FormatKind::Hex)
    } else {
        None
    }
}

fn validate_firmware_path(file_path: &str) -> anyhow::Result<()> {
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

fn send_done(tx: &mpsc::Sender<AppEvent>, success: bool, message: String) {
    tx.send(AppEvent::FlasherDone { success, message }).ok();
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

pub fn start_flash(state: &mut FlasherState, tx: mpsc::Sender<AppEvent>) -> anyhow::Result<()> {
    let chip_name = state.jtag_chip_name.trim().to_string();
    if chip_name.is_empty() {
        bail!("Chip name is empty (example: STM32F103C8)");
    }

    let file_path = state.jtag_file_path.trim().to_string();
    validate_firmware_path(&file_path)?;
    let format = detect_file_kind(&file_path).unwrap();

    let stop_flag = Arc::new(AtomicBool::new(false));
    state.stop_flag = Some(stop_flag.clone());

    spawn_jtag_thread(state.jtag_probe_idx, chip_name, file_path, format, tx, stop_flag);
    Ok(())
}

fn spawn_jtag_thread(
    probe_idx: usize,
    chip_name: String,
    file_path: String,
    format: flashing::FormatKind,
    tx: mpsc::Sender<AppEvent>,
    stop_flag: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let result = run_jtag(probe_idx, &chip_name, &file_path, format, &tx, &stop_flag);
        match result {
            Ok(()) => {
                send_done(&tx, true, "Flash complete!".into());
            }
            Err(e) => {
                send_done(&tx, false, e.to_string());
            }
        }
    });
}

fn run_jtag(
    probe_idx: usize,
    chip_name: &str,
    file_path: &str,
    format: flashing::FormatKind,
    tx: &mpsc::Sender<AppEvent>,
    stop_flag: &AtomicBool,
) -> anyhow::Result<()> {
    macro_rules! log {
        ($($arg:tt)*) => {
            tx.send(AppEvent::FlasherLog(format!($($arg)*))).ok();
        }
    }
    macro_rules! check_stop {
        () => {
            if stop_flag.load(Ordering::Relaxed) {
                bail!("Operation cancelled by user");
            }
        };
    }

    log!("Enumerating debug probes...");
    let probes = Lister::new().list_all();
    if probe_idx >= probes.len() {
        bail!(
            "Selected probe is no longer available (index {}, found {})",
            probe_idx,
            probes.len()
        );
    }

    let probe_info = &probes[probe_idx];
    log!("Using probe: {}", probe_info);
    tx.send(AppEvent::FlasherProgress(10)).ok();

    check_stop!();
    let probe = probe_info
        .open()
        .context("Failed to open probe. Check if another tool is using it.")?;

    check_stop!();
    log!("Attaching to target {}...", chip_name);
    let mut session = probe
        .attach(chip_name, Permissions::default())
        .with_context(|| format!("Failed to attach target '{}'", chip_name))?;
    tx.send(AppEvent::FlasherProgress(30)).ok();

    check_stop!();
    log!("Flashing {}... (Esc takes effect after this step completes)", file_path);
    flashing::download_file(&mut session, Path::new(file_path), format)
        .context("JTAG/SWD flash failed")?;

    check_stop!();

    tx.send(AppEvent::FlasherProgress(100)).ok();
    log!("Flash completed");

    Ok(())
}
