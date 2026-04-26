pub const MENU_ITEMS: &[&str] = &["Serial", "JTAG", "SWD", "Quit"];

pub struct HomeState {
    pub selected: usize,
}

impl HomeState {
    pub fn new() -> Self {
        Self { selected: 0 }
    }
}
