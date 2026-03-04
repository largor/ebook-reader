#[derive(Debug, Default)]
pub struct TocState {
    pub visible: bool,
    pub selected: usize,
}

impl TocState {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self, max: usize) {
        if self.selected + 1 < max {
            self.selected += 1;
        }
    }
}
