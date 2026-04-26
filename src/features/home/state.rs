pub const MENU_ITEMS: &[&str] = &["UART", "JTAG", "SWD", "I2C", "SPI", "Quit"];

pub struct HomeState {
    pub selected: usize,
}

impl HomeState {
    pub fn new() -> Self {
        Self { selected: 0 }
    }
}
