use crate::serial::{ISP_BAUD_PRESETS, SerialConfig};
use std::cell::Cell;
use std::sync::{Arc, atomic::AtomicBool};

#[derive(Debug, Clone)]
pub struct SerialMonitorRestore {
    pub port_name: String,
    pub serial_config: SerialConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlasherMethod {
    UsartIsp,
    Jtag,
    Swd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlasherSubScreen {
    MethodSelect,
    Config,
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IspConfigField {
    Port,
    BaudRate,
    BootMode,
    AutoProfile,
    FilePath,
}

impl IspConfigField {
    pub fn next(self) -> Self {
        match self {
            Self::Port => Self::BaudRate,
            Self::BaudRate => Self::BootMode,
            Self::BootMode => Self::AutoProfile,
            Self::AutoProfile => Self::FilePath,
            Self::FilePath => Self::Port,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Port => Self::FilePath,
            Self::BaudRate => Self::Port,
            Self::BootMode => Self::BaudRate,
            Self::AutoProfile => Self::BootMode,
            Self::FilePath => Self::AutoProfile,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IspBootMode {
    Manual,
    Auto,
}

impl IspBootMode {
    pub fn next(self) -> Self {
        match self {
            Self::Manual => Self::Auto,
            Self::Auto => Self::Manual,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::Auto => "Auto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IspAutoProfile {
    Standard,
    Inverted,
}

impl IspAutoProfile {
    pub fn next(self) -> Self {
        match self {
            Self::Standard => Self::Inverted,
            Self::Inverted => Self::Standard,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "BOOT0=High, RESET=Low",
            Self::Inverted => "BOOT0=Low, RESET=High",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JtagConfigField {
    Probe,
    ChipName,
    FilePath,
}

impl JtagConfigField {
    pub fn next(self) -> Self {
        match self {
            Self::Probe => Self::ChipName,
            Self::ChipName => Self::FilePath,
            Self::FilePath => Self::Probe,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Probe => Self::FilePath,
            Self::ChipName => Self::Probe,
            Self::FilePath => Self::ChipName,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwdConfigField {
    Probe,
    Speed,
    ConnectMode,
    Verify,
    ResetRun,
    BinBaseAddress,
    ChipPreset,
    ChipName,
    FilePath,
}

impl SwdConfigField {
    pub fn next(self) -> Self {
        match self {
            Self::Probe => Self::Speed,
            Self::Speed => Self::ConnectMode,
            Self::ConnectMode => Self::Verify,
            Self::Verify => Self::ResetRun,
            Self::ResetRun => Self::BinBaseAddress,
            Self::BinBaseAddress => Self::ChipPreset,
            Self::ChipPreset => Self::ChipName,
            Self::ChipName => Self::FilePath,
            Self::FilePath => Self::Probe,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Probe => Self::FilePath,
            Self::Speed => Self::Probe,
            Self::ConnectMode => Self::Speed,
            Self::Verify => Self::ConnectMode,
            Self::ResetRun => Self::Verify,
            Self::BinBaseAddress => Self::ResetRun,
            Self::ChipPreset => Self::BinBaseAddress,
            Self::ChipName => Self::ChipPreset,
            Self::FilePath => Self::ChipName,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwdConnectMode {
    Normal,
    UnderReset,
}

impl SwdConnectMode {
    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::UnderReset,
            Self::UnderReset => Self::Normal,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::UnderReset => "Under Reset",
        }
    }
}

pub const METHOD_ITEMS: &[&str] = &[
    "USART ISP  (Serial Download)",
    "JTAG       (Debug Probe)",
    "SWD        (Debug Probe)",
];

pub const SWD_SPEED_PRESETS: &[u32] = &[100, 400, 1000, 1800, 4000, 8000];

pub const SWD_CHIP_PRESETS: &[&str] = &[
    "Custom",
    "STM32F103C8",
    "STM32F103ZE",
    "STM32F407VE",
    "STM32F407ZG",
    "STM32F429ZI",
    "STM32F767ZI",
    "STM32H743ZI",
    "STM32G431CB",
    "STM32G474RE",
    "STM32L431RC",
    "STM32L476RG",
];

pub struct FlasherState {
    pub sub_screen: FlasherSubScreen,
    pub method: FlasherMethod,
    pub selected: usize,

    pub isp_port_list: Vec<String>,
    pub isp_port_idx: usize,
    pub isp_baud_idx: usize,
    pub isp_boot_mode: IspBootMode,
    pub isp_auto_profile: IspAutoProfile,
    pub isp_file_path: String,
    pub isp_file_cursor: usize,
    pub isp_field: IspConfigField,

    pub jtag_probe_list: Vec<String>,
    pub jtag_probe_idx: usize,
    pub jtag_chip_name: String,
    pub jtag_chip_cursor: usize,
    pub jtag_file_path: String,
    pub jtag_file_cursor: usize,
    pub jtag_field: JtagConfigField,

    pub swd_probe_list: Vec<String>,
    pub swd_probe_idx: usize,
    pub swd_speed_idx: usize,
    pub swd_connect_mode: SwdConnectMode,
    pub swd_verify: bool,
    pub swd_reset_run: bool,
    pub swd_bin_base_address: String,
    pub swd_bin_base_cursor: usize,
    pub swd_chip_preset_idx: usize,
    pub swd_chip_name: String,
    pub swd_chip_cursor: usize,
    pub swd_file_path: String,
    pub swd_file_cursor: usize,
    pub swd_field: SwdConfigField,

    pub log: Vec<String>,
    pub log_scroll: usize,
    pub log_visible_rows: Cell<u16>,
    pub progress_pct: Option<u8>,
    pub op_done: bool,
    pub op_ok: bool,
    pub cancel_armed: bool,
    pub stop_flag: Option<Arc<AtomicBool>>,
    pub serial_monitor_restore: Option<SerialMonitorRestore>,
}

impl FlasherState {
    pub fn new() -> Self {
        Self {
            sub_screen: FlasherSubScreen::MethodSelect,
            method: FlasherMethod::UsartIsp,
            selected: 0,

            isp_port_list: Vec::new(),
            isp_port_idx: 0,
            isp_baud_idx: 7,
            isp_boot_mode: IspBootMode::Manual,
            isp_auto_profile: IspAutoProfile::Standard,
            isp_file_path: String::new(),
            isp_file_cursor: 0,
            isp_field: IspConfigField::Port,

            jtag_probe_list: Vec::new(),
            jtag_probe_idx: 0,
            jtag_chip_name: String::new(),
            jtag_chip_cursor: 0,
            jtag_file_path: String::new(),
            jtag_file_cursor: 0,
            jtag_field: JtagConfigField::Probe,

            swd_probe_list: Vec::new(),
            swd_probe_idx: 0,
            swd_speed_idx: 2,
            swd_connect_mode: SwdConnectMode::Normal,
            swd_verify: true,
            swd_reset_run: true,
            swd_bin_base_address: "0x08000000".to_string(),
            swd_bin_base_cursor: 10,
            swd_chip_preset_idx: 0,
            swd_chip_name: String::new(),
            swd_chip_cursor: 0,
            swd_file_path: String::new(),
            swd_file_cursor: 0,
            swd_field: SwdConfigField::Probe,

            log: Vec::new(),
            log_scroll: 0,
            log_visible_rows: Cell::new(20),
            progress_pct: None,
            op_done: false,
            op_ok: false,
            cancel_armed: false,
            stop_flag: None,
            serial_monitor_restore: None,
        }
    }

    pub fn enter_config(&mut self) {
        self.method = match self.selected {
            0 => FlasherMethod::UsartIsp,
            1 => FlasherMethod::Jtag,
            _ => FlasherMethod::Swd,
        };
        self.sub_screen = FlasherSubScreen::Config;

        match self.method {
            FlasherMethod::UsartIsp => {
                self.isp_port_list = serialport::available_ports()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| p.port_name)
                    .collect();
                if self.isp_port_list.is_empty() {
                    self.isp_port_list.push("(no ports found)".to_string());
                }
                self.isp_port_idx = self
                    .isp_port_idx
                    .min(self.isp_port_list.len().saturating_sub(1));
                self.isp_field = IspConfigField::Port;
            }
            FlasherMethod::Jtag => {
                self.refresh_jtag_probes();
                self.jtag_field = JtagConfigField::Probe;
            }
            FlasherMethod::Swd => {
                self.refresh_swd_probes();
                self.swd_chip_preset_idx = self
                    .swd_chip_preset_idx
                    .min(SWD_CHIP_PRESETS.len().saturating_sub(1));
                self.swd_field = SwdConfigField::Probe;
            }
        }
    }

    pub fn enter_progress(&mut self) {
        self.sub_screen = FlasherSubScreen::Progress;
        self.log.clear();
        self.log_scroll = 0;
        self.progress_pct = None;
        self.op_done = false;
        self.op_ok = false;
        self.cancel_armed = false;
        self.stop_flag = None;
    }

    pub fn refresh_jtag_probes(&mut self) {
        self.jtag_probe_list = crate::flasher::common::list_probes();
        self.jtag_probe_idx = self
            .jtag_probe_idx
            .min(self.jtag_probe_list.len().saturating_sub(1));
    }

    pub fn refresh_swd_probes(&mut self) {
        self.swd_probe_list = crate::flasher::common::list_probes();
        self.swd_probe_idx = self
            .swd_probe_idx
            .min(self.swd_probe_list.len().saturating_sub(1));
    }

    pub fn plan_serial_monitor_restore(&mut self, serial_config: SerialConfig) {
        self.serial_monitor_restore = Some(SerialMonitorRestore {
            port_name: serial_config.port_name.clone(),
            serial_config,
        });
    }

    pub fn clear_serial_monitor_restore(&mut self) {
        self.serial_monitor_restore = None;
    }

    pub fn take_serial_monitor_restore(&mut self) -> Option<SerialMonitorRestore> {
        self.serial_monitor_restore.take()
    }

    pub fn isp_baud(&self) -> u32 {
        ISP_BAUD_PRESETS[self.isp_baud_idx]
    }

    pub fn swd_speed_khz(&self) -> u32 {
        SWD_SPEED_PRESETS[self.swd_speed_idx]
    }

    pub fn swd_chip_preset(&self) -> &'static str {
        SWD_CHIP_PRESETS[self.swd_chip_preset_idx]
    }

    pub fn swd_apply_chip_preset(&mut self) {
        if self.swd_chip_preset_idx == 0 {
            return;
        }

        self.swd_chip_name = SWD_CHIP_PRESETS[self.swd_chip_preset_idx].to_string();
        self.swd_chip_cursor = self.swd_chip_name.len();
    }

    pub fn scroll_log_up(&mut self, n: usize) {
        self.log_scroll = self.log_scroll.saturating_sub(n);
    }

    pub fn scroll_log_down(&mut self, n: usize) {
        let visible = self.log_visible_rows.get() as usize;
        let max_scroll = self.log.len().saturating_sub(visible);
        self.log_scroll = (self.log_scroll + n).min(max_scroll);
    }

    pub fn scroll_log_home(&mut self) {
        self.log_scroll = 0;
    }

    pub fn scroll_log_end(&mut self) {
        let visible = self.log_visible_rows.get() as usize;
        self.log_scroll = self.log.len().saturating_sub(visible);
    }

    pub fn isp_field_next(&mut self) {
        self.isp_field = self.isp_field.next();
        if self.isp_field == IspConfigField::AutoProfile && self.isp_boot_mode == IspBootMode::Manual {
            self.isp_field = self.isp_field.next();
        }
    }

    pub fn isp_field_prev(&mut self) {
        self.isp_field = self.isp_field.prev();
        if self.isp_field == IspConfigField::AutoProfile && self.isp_boot_mode == IspBootMode::Manual {
            self.isp_field = self.isp_field.prev();
        }
    }

    pub fn isp_file_input_char(&mut self, c: char) {
        self.isp_file_path.insert(self.isp_file_cursor, c);
        self.isp_file_cursor += c.len_utf8();
    }

    pub fn isp_file_backspace(&mut self) {
        if self.isp_file_cursor > 0 {
            let pos = self.isp_file_path[..self.isp_file_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.isp_file_path.remove(pos);
            self.isp_file_cursor = pos;
        }
    }

    pub fn jtag_chip_input_char(&mut self, c: char) {
        self.jtag_chip_name.insert(self.jtag_chip_cursor, c);
        self.jtag_chip_cursor += c.len_utf8();
    }

    pub fn jtag_chip_backspace(&mut self) {
        if self.jtag_chip_cursor > 0 {
            let pos = self.jtag_chip_name[..self.jtag_chip_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.jtag_chip_name.remove(pos);
            self.jtag_chip_cursor = pos;
        }
    }

    pub fn jtag_file_input_char(&mut self, c: char) {
        self.jtag_file_path.insert(self.jtag_file_cursor, c);
        self.jtag_file_cursor += c.len_utf8();
    }

    pub fn jtag_file_backspace(&mut self) {
        if self.jtag_file_cursor > 0 {
            let pos = self.jtag_file_path[..self.jtag_file_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.jtag_file_path.remove(pos);
            self.jtag_file_cursor = pos;
        }
    }

    pub fn swd_chip_input_char(&mut self, c: char) {
        self.swd_chip_name.insert(self.swd_chip_cursor, c);
        self.swd_chip_cursor += c.len_utf8();
    }

    pub fn swd_chip_backspace(&mut self) {
        if self.swd_chip_cursor > 0 {
            let pos = self.swd_chip_name[..self.swd_chip_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.swd_chip_name.remove(pos);
            self.swd_chip_cursor = pos;
        }
    }

    pub fn swd_file_input_char(&mut self, c: char) {
        self.swd_file_path.insert(self.swd_file_cursor, c);
        self.swd_file_cursor += c.len_utf8();
    }

    pub fn swd_file_backspace(&mut self) {
        if self.swd_file_cursor > 0 {
            let pos = self.swd_file_path[..self.swd_file_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.swd_file_path.remove(pos);
            self.swd_file_cursor = pos;
        }
    }

    pub fn swd_bin_base_input_char(&mut self, c: char) {
        self.swd_bin_base_address.insert(self.swd_bin_base_cursor, c);
        self.swd_bin_base_cursor += c.len_utf8();
    }

    pub fn swd_bin_base_backspace(&mut self) {
        if self.swd_bin_base_cursor > 0 {
            let pos = self.swd_bin_base_address[..self.swd_bin_base_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.swd_bin_base_address.remove(pos);
            self.swd_bin_base_cursor = pos;
        }
    }
}
