use crate::picker::{clamp_selection, FileEntry};
use crate::viewer::clamp_scroll;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Picker,
    Viewer,
}

#[derive(Debug)]
pub struct App {
    pub mode: Mode,
    pub files: Vec<FileEntry>,
    pub picker_index: usize,
    pub lines: Vec<String>,
    pub filename: String,
    pub scroll: usize,
    pub h_scroll: usize,
    pub wrap: bool,
    pub status_note: Option<String>,
    pub from_picker: bool,
}

impl App {
    pub fn scroll_by(&mut self, delta: isize, viewport_h: usize) {
        let total_hint = self.lines.len().max(1);
        let new = self.scroll as isize + delta;
        let clamped = new.clamp(0, isize::MAX) as usize;
        self.scroll = clamp_scroll(clamped, total_hint, viewport_h);
    }
    pub fn toggle_wrap(&mut self) {
        self.wrap = !self.wrap;
        self.h_scroll = 0;
    }
    pub fn move_picker(&mut self, delta: isize) {
        let n = self.picker_index as isize + delta;
        let n = n.clamp(0, isize::MAX) as usize;
        self.picker_index = clamp_selection(n, self.files.len());
    }
}

// src/app.rs tests
#[cfg(test)]
mod tests {
    use super::*;
    fn viewer_app(n: usize) -> App {
        App {
            mode: Mode::Viewer,
            files: vec![],
            picker_index: 0,
            lines: (0..n).map(|i| format!("line {i}")).collect(),
            filename: "t.txt".into(),
            scroll: 0,
            h_scroll: 0,
            wrap: true,
            status_note: None,
            from_picker: false,
        }
    }
    #[test]
    fn scroll_down_clamps_to_max() {
        let mut a = viewer_app(10);
        a.scroll_by(100, 5);
        assert_eq!(a.scroll, 5);
    }
    #[test]
    fn toggle_wrap_flips() {
        let mut a = viewer_app(2);
        assert!(a.wrap);
        a.toggle_wrap();
        assert!(!a.wrap);
    }
    #[test]
    fn picker_clamp_delegates() {
        let mut a = viewer_app(0);
        a.mode = Mode::Picker;
        a.files = vec![];
        a.move_picker(5);
        assert_eq!(a.picker_index, 0);
    }
}
