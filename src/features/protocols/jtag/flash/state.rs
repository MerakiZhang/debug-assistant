use crate::features::protocols::flashing::state::JtagConfigField;
use crate::ui::widgets;

pub struct JtagFlashState {
    pub probe_list: Vec<String>,
    pub probe_idx: usize,
    pub speed_idx: usize,
    pub verify: bool,
    pub reset_run: bool,
    pub bin_base_address: String,
    pub bin_base_cursor: usize,
    pub chip_preset_idx: usize,
    pub chip_name: String,
    pub chip_cursor: usize,
    pub file_path: String,
    pub file_cursor: usize,
    pub field: JtagConfigField,
}

impl JtagFlashState {
    pub fn new() -> Self {
        Self {
            probe_list: Vec::new(),
            probe_idx: 0,
            speed_idx: 2,
            verify: true,
            reset_run: true,
            bin_base_address: "0x08000000".to_string(),
            bin_base_cursor: 10,
            chip_preset_idx: 0,
            chip_name: String::new(),
            chip_cursor: 0,
            file_path: String::new(),
            file_cursor: 0,
            field: JtagConfigField::Probe,
        }
    }

    pub fn chip_input_char(&mut self, c: char) {
        widgets::text_input_char(&mut self.chip_name, &mut self.chip_cursor, c);
    }

    pub fn chip_backspace(&mut self) {
        widgets::text_backspace(&mut self.chip_name, &mut self.chip_cursor);
    }

    pub fn file_input_char(&mut self, c: char) {
        widgets::text_input_char(&mut self.file_path, &mut self.file_cursor, c);
    }

    pub fn file_backspace(&mut self) {
        widgets::text_backspace(&mut self.file_path, &mut self.file_cursor);
    }

    pub fn bin_base_input_char(&mut self, c: char) {
        widgets::text_input_char(&mut self.bin_base_address, &mut self.bin_base_cursor, c);
    }

    pub fn bin_base_backspace(&mut self) {
        widgets::text_backspace(&mut self.bin_base_address, &mut self.bin_base_cursor);
    }
}
