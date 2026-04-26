use chrono::Local;

use super::state::{Direction, LogEntry, SerialMonitorState};

const MAX_LOG_ENTRIES: usize = 5000;

impl SerialMonitorState {
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
