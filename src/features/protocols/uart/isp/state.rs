use crate::features::protocols::flashing::state::{IspBootMode, IspConfigField};
use crate::ui::widgets;

pub struct UsartIspState {
    pub port_list: Vec<String>,
    pub port_idx: usize,
    pub baud_idx: usize,
    pub boot_mode: IspBootMode,
    pub file_path: String,
    pub file_cursor: usize,
    pub field: IspConfigField,
}

impl UsartIspState {
    pub fn new() -> Self {
        Self {
            port_list: Vec::new(),
            port_idx: 0,
            baud_idx: 7,
            boot_mode: IspBootMode::Manual,
            file_path: String::new(),
            file_cursor: 0,
            field: IspConfigField::Port,
        }
    }

    pub fn file_input_char(&mut self, c: char) {
        widgets::text_input_char(&mut self.file_path, &mut self.file_cursor, c);
    }

    pub fn file_backspace(&mut self) {
        widgets::text_backspace(&mut self.file_path, &mut self.file_cursor);
    }

    pub fn file_cursor_left(&mut self) {
        widgets::text_cursor_left(&self.file_path, &mut self.file_cursor);
    }

    pub fn file_cursor_right(&mut self) {
        widgets::text_cursor_right(&self.file_path, &mut self.file_cursor);
    }

    pub fn file_cursor_home(&mut self) {
        widgets::text_cursor_home(&mut self.file_cursor);
    }

    pub fn file_cursor_end(&mut self) {
        widgets::text_cursor_end(&self.file_path, &mut self.file_cursor);
    }
}
