pub const MENU_ITEMS: &[&str] = &["Serial Monitor", "STM32 Flasher", "Quit"];

pub struct HomeState {
    pub selected: usize,
}

impl HomeState {
    pub fn new() -> Self {
        Self { selected: 0 }
    }
}
