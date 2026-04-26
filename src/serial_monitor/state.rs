use crate::event::AppEvent;
use crate::serial::{
    SerialConfig, BAUD_PRESETS, DATA_BITS_OPTIONS, FLOW_CONTROL_OPTIONS, PARITY_OPTIONS,
    STOP_BITS_OPTIONS,
};
use chrono::{DateTime, Local};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc};

const MAX_LOG_ENTRIES: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Receive,
    Send,
    ConfigPopup,
    HelpOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Ascii,
    Hex,
    Both,
}

impl DisplayMode {
    pub fn next(self) -> Self {
        match self {
            Self::Ascii => Self::Hex,
            Self::Hex => Self::Both,
            Self::Both => Self::Ascii,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Ascii => "ASCII",
            Self::Hex => "HEX",
            Self::Both => "BOTH",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewlineSuffix {
    None,
    CR,
    LF,
    CRLF,
}

impl NewlineSuffix {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::CR => "CR",
            Self::LF => "LF",
            Self::CRLF => "CRLF",
        }
    }
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Self::None => b"",
            Self::CR => b"\r",
            Self::LF => b"\n",
            Self::CRLF => b"\r\n",
        }
    }
    pub fn cycle(self) -> Self {
        match self {
            Self::None => Self::CR,
            Self::CR => Self::LF,
            Self::LF => Self::CRLF,
            Self::CRLF => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Rx,
    Tx,
    Status,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub raw: Vec<u8>,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigField {
    PortName,
    BaudRate,
    DataBits,
    StopBits,
    Parity,
    FlowControl,
}

impl ConfigField {
    pub fn next(self) -> Self {
        match self {
            Self::PortName => Self::BaudRate,
            Self::BaudRate => Self::DataBits,
            Self::DataBits => Self::StopBits,
            Self::StopBits => Self::Parity,
            Self::Parity => Self::FlowControl,
            Self::FlowControl => Self::PortName,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Self::PortName => Self::FlowControl,
            Self::BaudRate => Self::PortName,
            Self::DataBits => Self::BaudRate,
            Self::StopBits => Self::DataBits,
            Self::Parity => Self::StopBits,
            Self::FlowControl => Self::Parity,
        }
    }
}

pub struct SerialMonitorState {
    // connection
    pub connected: bool,
    pub serial_config: SerialConfig,
    pub serial_tx: Option<Sender<Vec<u8>>>,
    pub stop_flag: Option<Arc<AtomicBool>>,
    pub reader_thread: Option<std::thread::JoinHandle<()>>,
    // receive
    pub log: Vec<LogEntry>,
    pub log_scroll: usize,
    pub auto_scroll: bool,
    pub display_mode: DisplayMode,
    pub rx_visible_rows: Cell<u16>,
    // line buffer: accumulate bytes until \n or timeout
    pub rx_line_buf: Vec<u8>,
    pub rx_last_data: Option<std::time::Instant>,
    // send
    pub input_buf: String,
    pub cursor_pos: usize,
    pub send_history: Vec<String>,
    pub history_idx: Option<usize>,
    pub newline_suffix: NewlineSuffix,
    pub hex_send_mode: bool,
    // stats
    pub bytes_tx: u64,
    pub bytes_rx: u64,
    // UI
    pub focus: Focus,
    pub prev_focus: Focus,
    pub show_config: bool,
    pub show_help: bool,
    // config popup draft
    pub config_draft: SerialConfig,
    pub config_field: ConfigField,
    pub config_port_list: Vec<String>,
    pub config_port_idx: usize,
    pub config_baud_idx: usize,
    pub config_databits_idx: usize,
    pub config_stopbits_idx: usize,
    pub config_parity_idx: usize,
    pub config_flow_idx: usize,
}

impl SerialMonitorState {
    pub fn new() -> Self {
        Self {
            connected: false,
            serial_config: SerialConfig::default(),
            serial_tx: None,
            stop_flag: None,
            reader_thread: None,
            log: Vec::new(),
            log_scroll: 0,
            auto_scroll: true,
            display_mode: DisplayMode::Ascii,
            rx_visible_rows: Cell::new(20),
            rx_line_buf: Vec::new(),
            rx_last_data: None,
            input_buf: String::new(),
            cursor_pos: 0,
            send_history: Vec::new(),
            history_idx: None,
            newline_suffix: NewlineSuffix::LF,
            hex_send_mode: false,
            bytes_tx: 0,
            bytes_rx: 0,
            focus: Focus::Send,
            prev_focus: Focus::Send,
            show_config: false,
            show_help: false,
            config_draft: SerialConfig::default(),
            config_field: ConfigField::PortName,
            config_port_list: Vec::new(),
            config_port_idx: 0,
            config_baud_idx: 9,     // 115200
            config_databits_idx: 3, // Eight
            config_stopbits_idx: 0, // One
            config_parity_idx: 0,   // None
            config_flow_idx: 0,     // None
        }
    }

    pub fn push_rx(&mut self, data: Vec<u8>) {
        self.bytes_rx += data.len() as u64;
        self.rx_last_data = Some(std::time::Instant::now());
        self.rx_line_buf.extend_from_slice(&data);

        while let Some(pos) = self.rx_line_buf.iter().position(|&b| b == b'\n') {
            let raw: Vec<u8> = self.rx_line_buf.drain(..=pos).collect();
            self.commit_rx_line(raw);
        }

        if self.rx_line_buf.len() >= 1024 {
            let raw = std::mem::take(&mut self.rx_line_buf);
            self.commit_rx_line(raw);
        }

        self.ensure_auto_scroll();
    }

    pub fn flush_rx_buf(&mut self) {
        let timed_out = self
            .rx_last_data
            .map(|t| t.elapsed() > std::time::Duration::from_millis(100))
            .unwrap_or(false);
        if timed_out && !self.rx_line_buf.is_empty() {
            let raw = std::mem::take(&mut self.rx_line_buf);
            self.rx_last_data = None;
            self.commit_rx_line(raw);
            self.ensure_auto_scroll();
        }
    }

    fn commit_rx_line(&mut self, mut raw: Vec<u8>) {
        while matches!(raw.last(), Some(&b'\r') | Some(&b'\n')) {
            raw.pop();
        }
        if raw.is_empty() {
            return;
        }
        self.log.push(LogEntry {
            timestamp: Local::now(),
            raw,
            direction: Direction::Rx,
        });
        self.trim_log_if_needed();
    }

    pub fn push_tx(&mut self, data: Vec<u8>) {
        self.log.push(LogEntry {
            timestamp: Local::now(),
            raw: data,
            direction: Direction::Tx,
        });
        self.trim_log_if_needed();
        self.ensure_auto_scroll();
    }

    pub fn push_status(&mut self, msg: String) {
        self.log.push(LogEntry {
            timestamp: Local::now(),
            raw: msg.into_bytes(),
            direction: Direction::Status,
        });
        self.trim_log_if_needed();
        self.ensure_auto_scroll();
    }

    fn trim_log_if_needed(&mut self) {
        if self.log.len() > MAX_LOG_ENTRIES {
            let overflow = self.log.len() - MAX_LOG_ENTRIES;
            self.log.drain(0..overflow);
            self.log_scroll = self.log_scroll.saturating_sub(overflow);
        }
    }

    pub fn clear_log(&mut self) {
        self.log.clear();
        self.log_scroll = 0;
        self.rx_line_buf.clear();
        self.rx_last_data = None;
    }

    pub fn export_log_text(&self) -> String {
        self.log
            .iter()
            .map(format_log_entry_for_export)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn ensure_auto_scroll(&mut self) {
        if self.auto_scroll {
            let visible = self.rx_visible_rows.get() as usize;
            self.log_scroll = self.log.len().saturating_sub(visible);
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.log_scroll = self.log_scroll.saturating_sub(n);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, n: usize) {
        let visible = self.rx_visible_rows.get() as usize;
        let max_scroll = self.log.len().saturating_sub(visible);
        self.log_scroll = (self.log_scroll + n).min(max_scroll);
        if self.log_scroll >= max_scroll {
            self.auto_scroll = true;
        }
    }

    pub fn open_config_popup(&mut self) {
        self.config_draft = self.serial_config.clone();
        self.config_port_list = serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .collect();
        if self.config_port_list.is_empty() {
            self.config_port_list.push(String::from("(no ports found)"));
        }

        self.config_port_idx = self
            .config_port_list
            .iter()
            .position(|p| *p == self.serial_config.port_name)
            .unwrap_or(0);
        self.config_baud_idx = BAUD_PRESETS
            .iter()
            .position(|&b| b == self.serial_config.baud_rate)
            .unwrap_or(9);
        self.config_databits_idx = DATA_BITS_OPTIONS
            .iter()
            .position(|&d| d == self.serial_config.data_bits)
            .unwrap_or(3);
        self.config_stopbits_idx = STOP_BITS_OPTIONS
            .iter()
            .position(|&s| s == self.serial_config.stop_bits)
            .unwrap_or(0);
        self.config_parity_idx = PARITY_OPTIONS
            .iter()
            .position(|&p| p == self.serial_config.parity)
            .unwrap_or(0);
        self.config_flow_idx = FLOW_CONTROL_OPTIONS
            .iter()
            .position(|&f| f == self.serial_config.flow_control)
            .unwrap_or(0);

        self.config_field = ConfigField::PortName;
        self.prev_focus = self.focus;
        self.show_config = true;
        self.focus = Focus::ConfigPopup;
    }

    pub fn apply_config_and_close(&mut self, event_tx: Sender<AppEvent>) -> anyhow::Result<()> {
        if !self.config_port_list.is_empty() {
            self.config_draft.port_name = self.config_port_list[self.config_port_idx].clone();
        }
        self.config_draft.baud_rate = BAUD_PRESETS[self.config_baud_idx];
        self.config_draft.data_bits = DATA_BITS_OPTIONS[self.config_databits_idx];
        self.config_draft.stop_bits = STOP_BITS_OPTIONS[self.config_stopbits_idx];
        self.config_draft.parity = PARITY_OPTIONS[self.config_parity_idx];
        self.config_draft.flow_control = FLOW_CONTROL_OPTIONS[self.config_flow_idx];
        self.serial_config = self.config_draft.clone();

        if self.connected {
            self.disconnect();
        }
        self.connect(event_tx)?;

        self.show_config = false;
        self.focus = Focus::Send;
        Ok(())
    }

    pub fn cancel_config(&mut self) {
        self.show_config = false;
        self.focus = self.prev_focus;
    }

    pub fn connect(&mut self, event_tx: Sender<AppEvent>) -> anyhow::Result<()> {
        let (write_tx, stop, reader_handle) =
            crate::serial::spawn_serial_threads(&self.serial_config, event_tx)?;
        self.serial_tx = Some(write_tx);
        self.stop_flag = Some(stop);
        self.reader_thread = Some(reader_handle);
        self.connected = true;
        let msg = format!(
            "Connected: {} @ {} baud",
            self.serial_config.port_name, self.serial_config.baud_rate
        );
        self.push_status(msg);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.disconnect_with_status(true);
    }

    pub fn disconnect_with_status(&mut self, show_status: bool) {
        if self.connected {
            self.connected = false;
            if let Some(flag) = self.stop_flag.take() {
                flag.store(true, Ordering::Relaxed);
            }
            self.serial_tx = None;
            // Wait for reader thread to exit so the serial port is fully released
            if let Some(handle) = self.reader_thread.take() {
                let _ = handle.join();
            }
            if show_status {
                self.push_status("Disconnected".to_string());
            }
        }
    }

    pub fn prepare_current_input_bytes(&mut self) -> Option<Vec<u8>> {
        if self.input_buf.is_empty() {
            return None;
        }
        let payload: Vec<u8> = if self.hex_send_mode {
            match parse_hex_input(&self.input_buf) {
                Ok(b) => b,
                Err(e) => {
                    self.push_status(format!("Hex error: {}", e));
                    return None;
                }
            }
        } else {
            self.input_buf.as_bytes().to_vec()
        };

        let mut to_send = payload;
        to_send.extend_from_slice(self.newline_suffix.bytes());
        Some(to_send)
    }

    pub fn commit_sent_input(&mut self, sent: Vec<u8>) {
        let s = self.input_buf.clone();
        if self.send_history.last().map(|x| x != &s).unwrap_or(true) {
            self.send_history.push(s);
            if self.send_history.len() > 100 {
                self.send_history.remove(0);
            }
        }
        self.history_idx = None;
        self.input_buf.clear();
        self.cursor_pos = 0;
        self.bytes_tx += sent.len() as u64;
        self.push_tx(sent);
    }

    pub fn config_field_next_option(&mut self) {
        match self.config_field {
            ConfigField::PortName => {
                self.config_port_idx =
                    (self.config_port_idx + 1) % self.config_port_list.len().max(1)
            }
            ConfigField::BaudRate => {
                self.config_baud_idx = (self.config_baud_idx + 1) % BAUD_PRESETS.len()
            }
            ConfigField::DataBits => {
                self.config_databits_idx = (self.config_databits_idx + 1) % DATA_BITS_OPTIONS.len()
            }
            ConfigField::StopBits => {
                self.config_stopbits_idx = (self.config_stopbits_idx + 1) % STOP_BITS_OPTIONS.len()
            }
            ConfigField::Parity => {
                self.config_parity_idx = (self.config_parity_idx + 1) % PARITY_OPTIONS.len()
            }
            ConfigField::FlowControl => {
                self.config_flow_idx = (self.config_flow_idx + 1) % FLOW_CONTROL_OPTIONS.len()
            }
        }
    }

    pub fn config_field_prev_option(&mut self) {
        fn wrap_dec(val: usize, len: usize) -> usize {
            if val == 0 {
                len.saturating_sub(1)
            } else {
                val - 1
            }
        }
        match self.config_field {
            ConfigField::PortName => {
                self.config_port_idx =
                    wrap_dec(self.config_port_idx, self.config_port_list.len().max(1))
            }
            ConfigField::BaudRate => {
                self.config_baud_idx = wrap_dec(self.config_baud_idx, BAUD_PRESETS.len())
            }
            ConfigField::DataBits => {
                self.config_databits_idx =
                    wrap_dec(self.config_databits_idx, DATA_BITS_OPTIONS.len())
            }
            ConfigField::StopBits => {
                self.config_stopbits_idx =
                    wrap_dec(self.config_stopbits_idx, STOP_BITS_OPTIONS.len())
            }
            ConfigField::Parity => {
                self.config_parity_idx = wrap_dec(self.config_parity_idx, PARITY_OPTIONS.len())
            }
            ConfigField::FlowControl => {
                self.config_flow_idx = wrap_dec(self.config_flow_idx, FLOW_CONTROL_OPTIONS.len())
            }
        }
    }

    pub fn input_char(&mut self, c: char) {
        self.history_idx = None;
        self.input_buf.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn input_backspace(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let new_pos = self.input_buf[..self.cursor_pos]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.input_buf.remove(new_pos);
        self.cursor_pos = new_pos;
    }

    pub fn input_delete(&mut self) {
        if self.cursor_pos < self.input_buf.len() {
            self.input_buf.remove(self.cursor_pos);
        }
    }

    pub fn input_cursor_left(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        self.cursor_pos = self.input_buf[..self.cursor_pos]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    pub fn input_cursor_right(&mut self) {
        if self.cursor_pos < self.input_buf.len() {
            let c = self.input_buf[self.cursor_pos..].chars().next().unwrap();
            self.cursor_pos += c.len_utf8();
        }
    }

    pub fn input_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn input_cursor_end(&mut self) {
        self.cursor_pos = self.input_buf.len();
    }

    pub fn history_up(&mut self) {
        if self.send_history.is_empty() {
            return;
        }
        let idx = match self.history_idx {
            None => self.send_history.len() - 1,
            Some(i) => i.saturating_sub(1),
        };
        self.history_idx = Some(idx);
        self.input_buf = self.send_history[idx].clone();
        self.cursor_pos = self.input_buf.len();
    }

    pub fn history_down(&mut self) {
        match self.history_idx {
            None => {}
            Some(i) if i + 1 >= self.send_history.len() => {
                self.history_idx = None;
                self.input_buf.clear();
                self.cursor_pos = 0;
            }
            Some(i) => {
                let idx = i + 1;
                self.history_idx = Some(idx);
                self.input_buf = self.send_history[idx].clone();
                self.cursor_pos = self.input_buf.len();
            }
        }
    }
}

fn format_log_entry_for_export(entry: &LogEntry) -> String {
    let ts = entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f");
    match entry.direction {
        Direction::Status => format!("[{} STATUS] {}", ts, decode_export_text(&entry.raw)),
        Direction::Rx => format!("[{} RX] {}", ts, decode_export_text(&entry.raw)),
        Direction::Tx => format!("[{} TX] {}", ts, decode_export_text(&entry.raw)),
    }
}

fn decode_export_text(raw: &[u8]) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        match std::str::from_utf8(&raw[i..]) {
            Ok(s) => {
                for c in s.chars() {
                    match c {
                        '\r' => out.push_str("\\r"),
                        '\n' => out.push_str("\\n"),
                        '\t' => out.push_str("\\t"),
                        c if c.is_control() => out.push_str(&format!("[{:02X}]", c as u32)),
                        c => out.push(c),
                    }
                }
                break;
            }
            Err(e) => {
                let valid = std::str::from_utf8(&raw[i..i + e.valid_up_to()]).unwrap();
                for c in valid.chars() {
                    match c {
                        '\r' => out.push_str("\\r"),
                        '\n' => out.push_str("\\n"),
                        '\t' => out.push_str("\\t"),
                        c if c.is_control() => out.push_str(&format!("[{:02X}]", c as u32)),
                        c => out.push(c),
                    }
                }
                i += e.valid_up_to();
                let bad = e.error_len().unwrap_or(1);
                for &b in &raw[i..i + bad] {
                    out.push_str(&format!("\\x{:02X}", b));
                }
                i += bad;
            }
        }
    }
    out
}

fn parse_hex_input(s: &str) -> anyhow::Result<Vec<u8>> {
    s.split_whitespace()
        .map(|tok| {
            u8::from_str_radix(tok, 16).map_err(|_| anyhow::anyhow!("Invalid hex token: '{}'", tok))
        })
        .collect()
}
