use crate::features::protocols::jtag::flash::state::JtagFlashState;
use crate::features::protocols::swd::flash::state::SwdFlashState;
use crate::features::protocols::uart::isp::state::UsartIspState;
use crate::transport::serial::{SerialConfig, ISP_BAUD_PRESETS};
use std::cell::Cell;
use std::sync::{atomic::AtomicBool, Arc};

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
    Config,
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IspConfigField {
    Port,
    BaudRate,
    BootMode,
    FilePath,
}

impl IspConfigField {
    pub fn next(self) -> Self {
        match self {
            Self::Port => Self::BaudRate,
            Self::BaudRate => Self::BootMode,
            Self::BootMode => Self::FilePath,
            Self::FilePath => Self::Port,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Port => Self::FilePath,
            Self::BaudRate => Self::Port,
            Self::BootMode => Self::BaudRate,
            Self::FilePath => Self::BootMode,
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
pub enum JtagConfigField {
    Probe,
    Speed,
    Verify,
    ResetRun,
    BinBaseAddress,
    ChipPreset,
    ChipName,
    FilePath,
}

impl JtagConfigField {
    pub fn next(self) -> Self {
        match self {
            Self::Probe => Self::Speed,
            Self::Speed => Self::Verify,
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
            Self::Verify => Self::Speed,
            Self::ResetRun => Self::Verify,
            Self::BinBaseAddress => Self::ResetRun,
            Self::ChipPreset => Self::BinBaseAddress,
            Self::ChipName => Self::ChipPreset,
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

pub const SWD_SPEED_PRESETS: &[u32] = &[100, 400, 1000, 1800, 4000, 8000];
pub const JTAG_SPEED_PRESETS: &[u32] = SWD_SPEED_PRESETS;

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
pub const JTAG_CHIP_PRESETS: &[&str] = SWD_CHIP_PRESETS;

pub struct FlasherState {
    pub sub_screen: FlasherSubScreen,
    pub method: FlasherMethod,

    pub usart_isp: UsartIspState,
    pub jtag: JtagFlashState,
    pub swd: SwdFlashState,

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
            sub_screen: FlasherSubScreen::Config,
            method: FlasherMethod::UsartIsp,

            usart_isp: UsartIspState::new(),
            jtag: JtagFlashState::new(),
            swd: SwdFlashState::new(),

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

    pub fn enter_protocol_config(&mut self, method: FlasherMethod) {
        self.method = method;
        self.sub_screen = FlasherSubScreen::Config;

        match self.method {
            FlasherMethod::UsartIsp => {
                self.usart_isp.port_list = serialport::available_ports()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| p.port_name)
                    .collect();
                if self.usart_isp.port_list.is_empty() {
                    self.usart_isp
                        .port_list
                        .push("(no ports found)".to_string());
                }
                self.usart_isp.port_idx = self
                    .usart_isp
                    .port_idx
                    .min(self.usart_isp.port_list.len().saturating_sub(1));
                self.usart_isp.field = IspConfigField::Port;
            }
            FlasherMethod::Jtag => {
                self.refresh_jtag_probes();
                self.jtag.chip_preset_idx = self
                    .jtag
                    .chip_preset_idx
                    .min(JTAG_CHIP_PRESETS.len().saturating_sub(1));
                self.jtag.field = JtagConfigField::Probe;
            }
            FlasherMethod::Swd => {
                self.refresh_swd_probes();
                self.swd.chip_preset_idx = self
                    .swd
                    .chip_preset_idx
                    .min(SWD_CHIP_PRESETS.len().saturating_sub(1));
                self.swd.field = SwdConfigField::Probe;
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

    pub fn request_stop(&mut self) {
        if let Some(flag) = self.stop_flag.take() {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.cancel_armed = false;
    }

    pub fn refresh_jtag_probes(&mut self) {
        self.jtag.probe_list = crate::transport::probe::list_probes();
        self.jtag.probe_idx = self
            .jtag
            .probe_idx
            .min(self.jtag.probe_list.len().saturating_sub(1));
    }

    pub fn refresh_swd_probes(&mut self) {
        self.swd.probe_list = crate::transport::probe::list_probes();
        self.swd.probe_idx = self
            .swd
            .probe_idx
            .min(self.swd.probe_list.len().saturating_sub(1));
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
        ISP_BAUD_PRESETS[self.usart_isp.baud_idx]
    }

    pub fn swd_speed_khz(&self) -> u32 {
        SWD_SPEED_PRESETS[self.swd.speed_idx]
    }

    pub fn jtag_speed_khz(&self) -> u32 {
        JTAG_SPEED_PRESETS[self.jtag.speed_idx]
    }

    pub fn jtag_chip_preset(&self) -> &'static str {
        JTAG_CHIP_PRESETS[self.jtag.chip_preset_idx]
    }

    pub fn jtag_apply_chip_preset(&mut self) {
        if self.jtag.chip_preset_idx == 0 {
            return;
        }

        self.jtag.chip_name = JTAG_CHIP_PRESETS[self.jtag.chip_preset_idx].to_string();
        self.jtag.chip_cursor = self.jtag.chip_name.len();
    }

    pub fn swd_chip_preset(&self) -> &'static str {
        SWD_CHIP_PRESETS[self.swd.chip_preset_idx]
    }

    pub fn swd_apply_chip_preset(&mut self) {
        if self.swd.chip_preset_idx == 0 {
            return;
        }

        self.swd.chip_name = SWD_CHIP_PRESETS[self.swd.chip_preset_idx].to_string();
        self.swd.chip_cursor = self.swd.chip_name.len();
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
}
