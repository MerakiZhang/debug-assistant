use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use anyhow::{bail, Context};
use probe_rs::flashing;
use probe_rs::probe::list::Lister;
use probe_rs::probe::WireProtocol;
use probe_rs::Permissions;

use crate::app::event::AppEvent;
use crate::core::firmware::{
    detect_file_kind, format_for_download, parse_address, validate_firmware_path,
};
use crate::features::protocols::flashing::common::send_done;
use crate::features::protocols::flashing::state::{FlasherState, SwdConnectMode};

pub fn start_flash(state: &mut FlasherState, tx: mpsc::Sender<AppEvent>) -> anyhow::Result<()> {
    let chip_name = state.swd.chip_name.trim().to_string();
    if chip_name.is_empty() {
        bail!("Chip name is empty (example: STM32F103C8)");
    }

    let file_path = state.swd.file_path.trim().to_string();
    validate_firmware_path(&file_path)?;
    let file_kind = detect_file_kind(&file_path).unwrap();
    let bin_base_address = if file_kind == flashing::FormatKind::Bin {
        Some(parse_address(&state.swd.bin_base_address)?)
    } else {
        None
    };
    let format = format_for_download(&file_path, bin_base_address)?;

    let stop_flag = Arc::new(AtomicBool::new(false));
    state.stop_flag = Some(stop_flag.clone());

    let probe_idx = state.swd.probe_idx;
    let speed_khz = state.swd_speed_khz();
    let connect_mode = state.swd.connect_mode;
    let verify = state.swd.verify;
    let reset_run = state.swd.reset_run;

    std::thread::spawn(move || {
        let result = run_swd(
            probe_idx,
            &chip_name,
            &file_path,
            format,
            speed_khz,
            connect_mode,
            verify,
            reset_run,
            &tx,
            &stop_flag,
        );
        match result {
            Ok(()) => send_done(&tx, true, "Flash complete!".into()),
            Err(e) => send_done(&tx, false, e.to_string()),
        }
    });

    Ok(())
}

fn run_swd(
    probe_idx: usize,
    chip_name: &str,
    file_path: &str,
    format: flashing::Format,
    speed_khz: u32,
    connect_mode: SwdConnectMode,
    verify: bool,
    reset_run: bool,
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
    tx.send(AppEvent::FlasherProgress(8)).ok();

    check_stop!();
    let mut probe = probe_info
        .open()
        .context("Failed to open probe. Check if another tool is using it.")?;
    probe
        .select_protocol(WireProtocol::Swd)
        .context("Failed to switch probe to SWD mode")?;
    let actual_speed = probe
        .set_speed(speed_khz)
        .context("Failed to configure SWD speed")?;
    log!("SWD speed set to {} kHz", actual_speed);

    check_stop!();
    log!(
        "Attaching to target {} over SWD ({})...",
        chip_name,
        connect_mode.label()
    );
    let mut session = match connect_mode {
        SwdConnectMode::Normal => probe.attach(chip_name, Permissions::default()),
        SwdConnectMode::UnderReset => probe.attach_under_reset(chip_name, Permissions::default()),
    }
    .with_context(|| format!("Failed to attach target '{}'", chip_name))?;
    tx.send(AppEvent::FlasherProgress(28)).ok();

    check_stop!();
    let mut options = flashing::DownloadOptions::default();
    options.verify = verify;
    if verify {
        log!("Post-flash verify enabled");
    }
    log!(
        "Flashing {}... (Esc takes effect after this step completes)",
        file_path
    );
    flashing::download_file_with_options(&mut session, file_path, format, options)
        .context("SWD flash failed")?;
    tx.send(AppEvent::FlasherProgress(90)).ok();

    check_stop!();
    if reset_run {
        log!("Resetting target and resuming execution...");
        let mut core = session.core(0).context("Failed to access target core 0")?;
        core.reset()
            .context("Failed to reset target after flashing")?;
        core.run().context("Failed to resume target after reset")?;
    } else {
        log!("Leaving target in probe-rs default post-flash state");
    }

    tx.send(AppEvent::FlasherProgress(100)).ok();
    log!("Flash completed");
    Ok(())
}
