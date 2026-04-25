use crate::event::AppEvent;
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct SerialConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            stop_bits: StopBits::One,
            parity: Parity::None,
            flow_control: FlowControl::None,
        }
    }
}

pub const BAUD_PRESETS: &[u32] = &[
    300, 600, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600,
];

/// AN3155 valid baud rates for STM32 USART bootloader (1200–460800).
/// 300/600 are below the spec minimum; 921600 is unreliable with auto-baud detection.
pub const ISP_BAUD_PRESETS: &[u32] = &[
    1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800,
];

pub const DATA_BITS_OPTIONS: &[DataBits] = &[
    DataBits::Five,
    DataBits::Six,
    DataBits::Seven,
    DataBits::Eight,
];

pub const STOP_BITS_OPTIONS: &[StopBits] = &[StopBits::One, StopBits::Two];

pub const PARITY_OPTIONS: &[Parity] = &[Parity::None, Parity::Even, Parity::Odd];

pub const FLOW_CONTROL_OPTIONS: &[FlowControl] = &[
    FlowControl::None,
    FlowControl::Software,
    FlowControl::Hardware,
];

pub fn data_bits_label(d: DataBits) -> &'static str {
    match d {
        DataBits::Five => "5",
        DataBits::Six => "6",
        DataBits::Seven => "7",
        DataBits::Eight => "8",
    }
}

pub fn stop_bits_label(s: StopBits) -> &'static str {
    match s {
        StopBits::One => "1",
        StopBits::Two => "2",
    }
}

pub fn parity_label(p: Parity) -> &'static str {
    match p {
        Parity::None => "None",
        Parity::Even => "Even",
        Parity::Odd => "Odd",
    }
}

pub fn parity_short(p: Parity) -> &'static str {
    match p {
        Parity::None => "N",
        Parity::Even => "E",
        Parity::Odd => "O",
    }
}

pub fn flow_control_label(f: FlowControl) -> &'static str {
    match f {
        FlowControl::None => "None",
        FlowControl::Software => "XON/XOFF",
        FlowControl::Hardware => "RTS/CTS",
    }
}

pub fn spawn_serial_threads(
    config: &SerialConfig,
    event_tx: Sender<AppEvent>,
) -> anyhow::Result<(Sender<Vec<u8>>, Arc<AtomicBool>)> {
    let port = serialport::new(&config.port_name, config.baud_rate)
        .data_bits(config.data_bits)
        .stop_bits(config.stop_bits)
        .parity(config.parity)
        .flow_control(config.flow_control)
        .timeout(Duration::from_millis(100))
        .open()?;

    let writer_port = port.try_clone()?;
    let stop = Arc::new(AtomicBool::new(false));

    // Reader thread
    let stop_r = stop.clone();
    let tx_r = event_tx.clone();
    std::thread::spawn(move || {
        let mut port = port;
        let mut buf = [0u8; 1024];
        loop {
            if stop_r.load(Ordering::Relaxed) {
                break;
            }
            match port.read(&mut buf) {
                Ok(n) if n == 0 => continue,
                Ok(n) => {
                    tx_r.send(AppEvent::SerialData(buf[..n].to_vec())).ok();
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    tx_r.send(AppEvent::SerialError(e.to_string())).ok();
                    break;
                }
            }
        }
    });

    // Writer thread
    let (write_tx, write_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let tx_w = event_tx;
    std::thread::spawn(move || {
        let mut port = writer_port;
        while let Ok(bytes) = write_rx.recv() {
            if let Err(e) = port.write_all(&bytes) {
                tx_w.send(AppEvent::SerialError(e.to_string())).ok();
                break;
            }
        }
    });

    Ok((write_tx, stop))
}
