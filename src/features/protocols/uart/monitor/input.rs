use super::state::SerialMonitorState;

impl SerialMonitorState {
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

fn parse_hex_input(s: &str) -> anyhow::Result<Vec<u8>> {
    s.split_whitespace()
        .map(|tok| {
            u8::from_str_radix(tok, 16).map_err(|_| anyhow::anyhow!("Invalid hex token: '{}'", tok))
        })
        .collect()
}
