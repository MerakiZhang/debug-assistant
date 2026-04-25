mod protocol;

use super::state::{FlasherState, IspAutoProfile, IspBootMode};
use crate::event::AppEvent;
use anyhow::{bail, Context};
use ihex::Record;
use serialport::SerialPort;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

const CHUNK_SIZE: usize = 256;
const FLASH_BASE: u32 = 0x0800_0000;
const RESET_PULSE_MS: u64 = 80;
const BOOT_SETTLE_MS: u64 = 80;
const POST_RESET_DELAY_MS: u64 = 120;

#[derive(Debug, Clone)]
struct FirmwareSegment {
    address: u32,
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
struct FirmwareImage {
    kind_label: &'static str,
    segments: Vec<FirmwareSegment>,
    total_bytes: usize,
}

#[derive(Debug, Clone, Copy)]
struct IspBootConfig {
    mode: IspBootMode,
    auto_profile: IspAutoProfile,
}

struct SessionLogGuard {
    tx: mpsc::Sender<AppEvent>,
    port_name: String,
    active: bool,
}

impl SessionLogGuard {
    fn new(tx: mpsc::Sender<AppEvent>, port_name: String) -> Self {
        Self {
            tx,
            port_name,
            active: false,
        }
    }

    fn mark_open(&mut self) {
        self.active = true;
    }
}

impl Drop for SessionLogGuard {
    fn drop(&mut self) {
        if self.active {
            self.tx
                .send(AppEvent::FlasherLog(format!(
                    "Serial session closed on {}.",
                    self.port_name
                )))
                .ok();
        }
    }
}

fn is_supported_file(file_path: &str) -> bool {
    let lower = file_path.to_ascii_lowercase();
    lower.ends_with(".bin") || lower.ends_with(".hex")
}

fn validate_firmware_path(file_path: &str) -> anyhow::Result<()> {
    if file_path.is_empty() {
        bail!("File path is empty");
    }
    if !is_supported_file(file_path) {
        bail!("Only .bin/.hex files are supported (got: {})", file_path);
    }
    if !Path::new(file_path).exists() {
        bail!("File not found: {}", file_path);
    }
    Ok(())
}

/// Validate config and spawn the ISP flash thread. Returns Err if config is invalid.
pub fn start_flash(state: &mut FlasherState, tx: mpsc::Sender<AppEvent>) -> anyhow::Result<()> {
    let port_name = if state.isp_port_list.is_empty() {
        bail!("No serial port selected");
    } else {
        state.isp_port_list[state.isp_port_idx].clone()
    };

    if port_name == "(no ports found)" {
        bail!("No serial ports available on this system");
    }

    let file_path = state.isp_file_path.trim().to_string();
    validate_firmware_path(&file_path)?;

    let baud = state.isp_baud();
    let boot = IspBootConfig {
        mode: state.isp_boot_mode,
        auto_profile: state.isp_auto_profile,
    };
    let stop_flag = Arc::new(AtomicBool::new(false));
    state.stop_flag = Some(stop_flag.clone());

    spawn_isp_thread(port_name, baud, file_path, boot, tx, stop_flag);
    Ok(())
}

fn spawn_isp_thread(
    port_name: String,
    baud_rate: u32,
    file_path: String,
    boot: IspBootConfig,
    tx: mpsc::Sender<AppEvent>,
    stop_flag: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let result = run_isp(&port_name, baud_rate, &file_path, boot, &tx, &stop_flag);
        match result {
            Ok(()) => {
                tx.send(AppEvent::FlasherDone {
                    success: true,
                    message: "Flash complete!".into(),
                })
                .ok();
            }
            Err(e) => {
                tx.send(AppEvent::FlasherDone {
                    success: false,
                    message: e.to_string(),
                })
                .ok();
            }
        }
    });
}

fn run_isp(
    port_name: &str,
    baud_rate: u32,
    file_path: &str,
    boot: IspBootConfig,
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

    let mut session_guard = SessionLogGuard::new(tx.clone(), port_name.to_string());

    log!("Loading firmware: {}", file_path);
    let firmware = load_firmware_image(file_path)?;
    log!(
        "Parsed {} image: {} bytes across {} segment(s)",
        firmware.kind_label,
        firmware.total_bytes,
        firmware.segments.len()
    );

    check_stop!();

    log!("Opening {} at {} baud (8E1)...", port_name, baud_rate);
    let mut port: Box<dyn SerialPort> = serialport::new(port_name, baud_rate)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::Even)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(500))
        .open()
        .context("Failed to open serial port")?;
    session_guard.mark_open();
    log!("Serial session opened on {}.", port_name);

    check_stop!();

    enter_bootloader(port.as_mut(), boot, tx)?;

    check_stop!();

    let mut erase_done = false;

    // Find the selected baud rate in ISP_BAUD_PRESETS and step down on failure.
    // Each rate gets 2 attempts; on failure the port steps to the next lower rate.
    let start_idx = crate::serial::ISP_BAUD_PRESETS
        .iter()
        .position(|&b| b == baud_rate)
        .unwrap_or(0);

    'sync: for rate_idx in (0..=start_idx).rev() {
        let try_baud = crate::serial::ISP_BAUD_PRESETS[rate_idx];
        let is_first_rate = rate_idx == start_idx;

        if !is_first_rate {
            log!("Stepping down to {} baud...", try_baud);
            port.set_baud_rate(try_baud)?;
        }

        for attempt in 0..2 {
            check_stop!();

            // Bootloader was already entered before this loop for the first rate,
            // first attempt. Every other case needs a fresh entry.
            if !is_first_rate || attempt > 0 {
                if attempt > 0 {
                    log!(
                        "Retry {}/2 at {} baud: re-entering bootloader...",
                        attempt + 1,
                        try_baud
                    );
                } else {
                    log!("Entering bootloader at {} baud...", try_baud);
                }
                enter_bootloader(port.as_mut(), boot, tx)?;
                check_stop!();
            }

            log!(
                "Synchronizing at {} baud (attempt {}/2)...",
                try_baud,
                attempt + 1
            );
            if protocol::sync(port.as_mut(), 3).is_ok() {
                log!("Synchronized at {} baud!", try_baud);

                match protocol::get_id(port.as_mut()) {
                    Ok(id) => {
                        log!("Chip ID: 0x{:04X}", id);

                        check_stop!();

                        log!("Erasing flash (mass erase)...");
                        match protocol::extended_erase_all(port.as_mut(), || {
                            tx.send(AppEvent::FlasherProgress(5)).ok();
                            !stop_flag.load(Ordering::Relaxed)
                        }) {
                            Ok(()) => {
                                log!("Erase complete");
                                tx.send(AppEvent::FlasherProgress(10)).ok();
                                erase_done = true;
                                break 'sync;
                            }
                            Err(err) if err.to_string().contains("ExtendedErase command rejected") => {
                                log!(
                                    "ExtendedErase setup failed at {} baud: {}. Re-entering bootloader or stepping down.",
                                    try_baud,
                                    err
                                );
                            }
                            Err(err) => return Err(err),
                        }
                    }
                    Err(err) => {
                        log!(
                            "GetID failed at {} baud: {}. Re-entering bootloader or stepping down.",
                            try_baud,
                            err
                        );
                    }
                }
            }
        }

        log!("Sync/GetID/erase-setup failed at {} baud.", try_baud);
    }

    if !erase_done {
        let hint = match boot.mode {
            IspBootMode::Manual => {
                "Hint: ensure BOOT0=HIGH and target has been reset into ROM bootloader."
            }
            IspBootMode::Auto => {
                "Hint: try switching Auto Mode profile, check RTS->BOOT0 and DTR->RESET wiring."
            }
        };
        bail!("Sync/GetID/erase-setup failed after multiple retries. {}", hint);
    }

    check_stop!();

    check_stop!();

    log!("Writing {} bytes...", firmware.total_bytes);
    let mut written = 0usize;
    for segment in &firmware.segments {
        log!(
            "Writing segment at 0x{:08X} ({} bytes)",
            segment.address,
            segment.data.len()
        );
        for (chunk_idx, chunk) in segment.data.chunks(CHUNK_SIZE).enumerate() {
            check_stop!();

            let addr = segment.address + (chunk_idx * CHUNK_SIZE) as u32;
            protocol::write_chunk(port.as_mut(), addr, chunk)
                .with_context(|| format!("Write failed at 0x{:08X}", addr))?;

            written += chunk.len();
            let pct = (10 + written * 89 / firmware.total_bytes) as u8;
            tx.send(AppEvent::FlasherProgress(pct)).ok();
        }
    }

    log!("Write complete ({} bytes)", written);
    tx.send(AppEvent::FlasherProgress(99)).ok();

    check_stop!();

    log!("Starting application at 0x{:08X}...", FLASH_BASE);
    protocol::go(port.as_mut(), FLASH_BASE)?;
    log!("Done!");

    Ok(())
}

fn enter_bootloader(
    port: &mut dyn SerialPort,
    boot: IspBootConfig,
    tx: &mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    match boot.mode {
        IspBootMode::Manual => {
            tx.send(AppEvent::FlasherLog(
                "Manual boot mode: set BOOT0 high and reset the target before sync.".into(),
            ))
            .ok();
            Ok(())
        }
        IspBootMode::Auto => auto_enter_bootloader(port, boot.auto_profile, tx),
    }
}

fn auto_enter_bootloader(
    port: &mut dyn SerialPort,
    profile: IspAutoProfile,
    tx: &mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    let (boot_asserted, reset_asserted, profile_label) = match profile {
        IspAutoProfile::Standard => (true, true, "BOOT0=High, RESET=Low"),
        IspAutoProfile::Inverted => (false, false, "BOOT0=Low, RESET=High"),
    };

    tx.send(AppEvent::FlasherLog(format!(
        "Auto boot mode: driving RTS->BOOT0 and DTR->RESET with profile '{}'...",
        profile_label
    )))
    .ok();

    // Assume RTS controls BOOT0 and DTR controls RESET through an inverter stage.
    set_boot0(port, !boot_asserted).context("Failed to set BOOT0 idle level")?;
    set_reset(port, !reset_asserted).context("Failed to set RESET idle level")?;
    std::thread::sleep(Duration::from_millis(30));

    set_boot0(port, boot_asserted).context("Failed to assert BOOT0")?;
    set_reset(port, reset_asserted).context("Failed to assert RESET")?;
    std::thread::sleep(Duration::from_millis(RESET_PULSE_MS));

    set_reset(port, !reset_asserted).context("Failed to release RESET")?;
    std::thread::sleep(Duration::from_millis(BOOT_SETTLE_MS));

    set_boot0(port, !boot_asserted).context("Failed to release BOOT0")?;
    std::thread::sleep(Duration::from_millis(POST_RESET_DELAY_MS));

    Ok(())
}

fn set_boot0(port: &mut dyn SerialPort, level: bool) -> anyhow::Result<()> {
    port.write_request_to_send(level)
        .map_err(anyhow::Error::from)
}

fn set_reset(port: &mut dyn SerialPort, level: bool) -> anyhow::Result<()> {
    port.write_data_terminal_ready(level)
        .map_err(anyhow::Error::from)
}

fn load_firmware_image(file_path: &str) -> anyhow::Result<FirmwareImage> {
    let lower = file_path.to_ascii_lowercase();
    if lower.ends_with(".bin") {
        load_bin_image(file_path)
    } else if lower.ends_with(".hex") {
        load_hex_image(file_path)
    } else {
        bail!("Unsupported file extension: {}", file_path)
    }
}

fn load_bin_image(file_path: &str) -> anyhow::Result<FirmwareImage> {
    let firmware = std::fs::read(file_path).context("Failed to read firmware file")?;
    if firmware.is_empty() {
        bail!("Firmware file is empty");
    }

    Ok(FirmwareImage {
        kind_label: "BIN",
        total_bytes: firmware.len(),
        segments: vec![FirmwareSegment {
            address: FLASH_BASE,
            data: firmware,
        }],
    })
}

fn load_hex_image(file_path: &str) -> anyhow::Result<FirmwareImage> {
    let text = std::fs::read_to_string(file_path).context("Failed to read HEX file")?;

    let mut upper_linear = 0u32;
    let mut upper_segment = 0u32;
    let mut use_linear = true;
    let mut chunks: Vec<(u32, Vec<u8>)> = Vec::new();

    for (line_idx, record) in ihex::Reader::new(&text).enumerate() {
        let record = record.with_context(|| format!("HEX parse error at line {}", line_idx + 1))?;
        match record {
            Record::Data { offset, value } => {
                if value.is_empty() {
                    continue;
                }

                let base = if use_linear {
                    upper_linear << 16
                } else {
                    upper_segment << 4
                };
                let address = base + u32::from(offset);

                if address < FLASH_BASE {
                    bail!(
                        "HEX contains address below flash region: 0x{:08X} (minimum 0x{:08X})",
                        address,
                        FLASH_BASE
                    );
                }

                match chunks.last_mut() {
                    Some((last_addr, last_data))
                        if *last_addr + last_data.len() as u32 == address =>
                    {
                        last_data.extend_from_slice(&value);
                    }
                    _ => chunks.push((address, value)),
                }
            }
            Record::ExtendedLinearAddress(addr) => {
                upper_linear = u32::from(addr);
                use_linear = true;
            }
            Record::ExtendedSegmentAddress(addr) => {
                upper_segment = u32::from(addr);
                use_linear = false;
            }
            Record::EndOfFile
            | Record::StartLinearAddress(_)
            | Record::StartSegmentAddress { .. } => {}
        }
    }

    if chunks.is_empty() {
        bail!("HEX file does not contain any writable flash data");
    }

    chunks.sort_by_key(|(address, _)| *address);
    for pair in chunks.windows(2) {
        let (addr_a, data_a) = (&pair[0].0, &pair[0].1);
        let (addr_b, _) = (&pair[1].0, &pair[1].1);
        let end_a = *addr_a + data_a.len() as u32;
        if *addr_b < end_a {
            bail!(
                "HEX contains overlapping ranges near 0x{:08X} and 0x{:08X}",
                addr_a,
                addr_b
            );
        }
    }

    let segments: Vec<FirmwareSegment> = chunks
        .into_iter()
        .map(|(address, data)| FirmwareSegment { address, data })
        .collect();
    let total_bytes = segments.iter().map(|segment| segment.data.len()).sum();

    Ok(FirmwareImage {
        kind_label: "HEX",
        segments,
        total_bytes,
    })
}
