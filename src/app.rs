use crate::picker::{clamp_selection, FileEntry};

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
    pub viewport_h: usize,
}

impl App {
    pub fn scroll_by(&mut self, delta: isize, _viewport_h: usize) {
        self.scroll = self.scroll.saturating_add_signed(delta);
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

use anyhow::{Context, Result};
use std::path::Path;

impl App {
    pub fn new_picker(files: Vec<FileEntry>) -> Self {
        Self {
            mode: Mode::Picker,
            files,
            picker_index: 0,
            lines: vec![],
            filename: String::new(),
            scroll: 0,
            h_scroll: 0,
            wrap: true,
            status_note: None,
            from_picker: false,
            viewport_h: 20,
        }
    }
    pub fn load_file(path: &Path, from_picker: bool) -> Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("cannot open {}", path.display()))?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        let lossy = String::from_utf8(bytes).is_err();
        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
        Ok(Self {
            mode: Mode::Viewer,
            files: vec![],
            picker_index: 0,
            lines,
            filename: path.display().to_string(),
            scroll: 0,
            h_scroll: 0,
            wrap: true,
            status_note: lossy.then(|| "[lossy UTF-8]".to_string()),
            from_picker,
            viewport_h: 20,
        })
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
            viewport_h: 20,
        }
    }
    #[test]
    fn scroll_down_clamps_to_max() {
        // scroll_by defers clamping to render(); it only saturates.
        let mut a = viewer_app(10);
        a.scroll_by(100, 5);
        assert_eq!(a.scroll, 100);
        // Underflow saturates at 0.
        a.scroll_by(-200, 5);
        assert_eq!(a.scroll, 0);
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
