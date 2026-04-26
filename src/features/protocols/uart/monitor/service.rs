use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;

use crate::app::event::AppEvent;
use crate::features::protocols::uart::monitor::state::{Focus, SerialMonitorState};
use crate::transport::serial::{
    BAUD_PRESETS, DATA_BITS_OPTIONS, FLOW_CONTROL_OPTIONS, PARITY_OPTIONS, STOP_BITS_OPTIONS,
};

impl SerialMonitorState {
    pub fn refresh_port_list(&mut self) {
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
            .unwrap_or(7);
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
    }

    pub fn apply_setup(&mut self, event_tx: Sender<AppEvent>) -> anyhow::Result<()> {
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

        self.focus = Focus::Send;
        Ok(())
    }

    pub fn connect(&mut self, event_tx: Sender<AppEvent>) -> anyhow::Result<()> {
        let (write_tx, stop, reader_handle) =
            crate::transport::serial::spawn_serial_threads(&self.serial_config, event_tx)?;
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
            if let Some(handle) = self.reader_thread.take() {
                let _ = handle.join();
            }
            if show_status {
                self.push_status("Disconnected".to_string());
            }
        }
    }
}
