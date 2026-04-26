use crate::app::event::AppEvent;
use crate::core::firmware::{self, FLASH_BASE};
use crate::features::protocols::flashing::state::{FlasherState, IspBootMode};
use crate::features::protocols::uart::isp::protocol;
use anyhow::{bail, Context};
use serialport::SerialPort;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

const CHUNK_SIZE: usize = 256;
const RESET_PULSE_MS: u64 = 80;
const BOOT_SETTLE_MS: u64 = 80;
const POST_RESET_DELAY_MS: u64 = 120;

#[derive(Debug, Clone, Copy)]
struct IspBootConfig {
    mode: IspBootMode,
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

/// Validate config and spawn the ISP flash thread. Returns Err if config is invalid.
pub fn start_flash(state: &mut FlasherState, tx: mpsc::Sender<AppEvent>) -> anyhow::Result<()> {
    let port_name = if state.usart_isp.port_list.is_empty() {
        bail!("No serial port selected");
    } else {
        state.usart_isp.port_list[state.usart_isp.port_idx].clone()
    };

    if port_name == "(no ports found)" {
        bail!("No serial ports available on this system");
    }

    let file_path = state.usart_isp.file_path.trim().to_string();
    firmware::validate_firmware_path(&file_path)?;

    let baud = state.isp_baud();
    let boot = IspBootConfig {
        mode: state.usart_isp.boot_mode,
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
    let firmware = firmware::load_firmware_image(file_path)?;
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
    let start_idx = crate::transport::serial::ISP_BAUD_PRESETS
        .iter()
        .position(|&b| b == baud_rate)
        .unwrap_or(0);

    'sync: for rate_idx in (0..=start_idx).rev() {
        let try_baud = crate::transport::serial::ISP_BAUD_PRESETS[rate_idx];
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
                            Err(err)
                                if err.to_string().contains("ExtendedErase command rejected") =>
                            {
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
            IspBootMode::Auto => "Hint: check RTS->BOOT0 and DTR->RESET wiring.",
        };
        bail!(
            "Sync/GetID/erase-setup failed after multiple retries. {}",
            hint
        );
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
        IspBootMode::Auto => auto_enter_bootloader(port, tx),
    }
}

fn auto_enter_bootloader(
    port: &mut dyn SerialPort,
    tx: &mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    let boot_asserted = true;
    let reset_asserted = true;

    tx.send(AppEvent::FlasherLog(
        "Auto boot mode: driving RTS->BOOT0 high and pulsing DTR->RESET low...".into(),
    ))
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

fn set_reset(port: &mut dyn SerialPort, level: bool) -> anyhow::Result<()> {
    port.write_data_terminal_ready(level)
        .map_err(anyhow::Error::from)
}

fn set_boot0(port: &mut dyn SerialPort, level: bool) -> anyhow::Result<()> {
    port.write_request_to_send(level)
        .map_err(anyhow::Error::from)
}
