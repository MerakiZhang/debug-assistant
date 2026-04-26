use crate::transport::serial::{
    SerialConfig, BAUD_PRESETS, DATA_BITS_OPTIONS, FLOW_CONTROL_OPTIONS, PARITY_OPTIONS,
    STOP_BITS_OPTIONS,
};
use chrono::{DateTime, Local};
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc::Sender, Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Setup,
    Receive,
    Send,
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
            show_help: false,
            config_draft: SerialConfig::default(),
            config_field: ConfigField::PortName,
            config_port_list: Vec::new(),
            config_port_idx: 0,
            config_baud_idx: 7,     // 115200
            config_databits_idx: 3, // Eight
            config_stopbits_idx: 0, // One
            config_parity_idx: 0,   // None
            config_flow_idx: 0,     // None
        }
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

    pub fn focus_setup(&mut self) {
        if self.focus == Focus::Setup {
            return;
        }
        self.refresh_port_list();
        self.config_field = ConfigField::PortName;
        self.prev_focus = self.focus;
        self.focus = Focus::Setup;
    }

    pub fn cancel_setup(&mut self) {
        self.focus = Focus::Send;
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
}
