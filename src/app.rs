use crate::highlight::{self, Lang};
use crate::picker::{clamp_selection, FileEntry};
use ratatui::text::Span;

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
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub lang: Option<Lang>,
    pub highlighted: Option<Vec<Vec<Span<'static>>>>,
    // Theme used at load for `highlighted`; reserved for future re-highlight.
    #[allow(dead_code)]
    pub theme: highlight::Theme,
}

impl App {
    pub fn toggle_wrap(&mut self) {
        self.wrap = !self.wrap;
        self.h_scroll = 0;
    }
    pub fn move_picker(&mut self, delta: isize) {
        let n = self.picker_index as isize + delta;
        let n = n.clamp(0, isize::MAX) as usize;
        self.picker_index = clamp_selection(n, self.files.len());
    }
    fn clamp_col(&self) -> usize {
        self.lines
            .get(self.cursor_line)
            .map(|l| l.chars().count())
            .unwrap_or(0)
    }
    pub fn move_cursor_line(&mut self, delta: isize) {
        if self.lines.is_empty() {
            self.cursor_line = 0;
            self.cursor_col = 0;
            return;
        }
        let n = (self.cursor_line as isize + delta).clamp(0, isize::MAX) as usize;
        self.cursor_line = n.min(self.lines.len() - 1);
        self.cursor_col = self.cursor_col.min(self.clamp_col());
    }
    pub fn move_cursor_col(&mut self, delta: isize) {
        let max = self.clamp_col();
        let n = (self.cursor_col as isize + delta).clamp(0, isize::MAX) as usize;
        self.cursor_col = n.min(max);
    }
    /// Keep cursor's display row inside [scroll, scroll+viewport).
    pub fn ensure_cursor_visible(&mut self, display_row: usize, viewport: usize) {
        if viewport == 0 {
            return;
        }
        if display_row < self.scroll {
            self.scroll = display_row;
        } else if display_row >= self.scroll + viewport {
            self.scroll = display_row + 1 - viewport;
        }
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
            cursor_line: 0,
            cursor_col: 0,
            lang: None,
            highlighted: None,
            theme: highlight::Theme::Dark,
        }
    }
    pub fn load_file(path: &Path, from_picker: bool) -> Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("cannot open {}", path.display()))?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        let lossy = String::from_utf8(bytes).is_err();
        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
        let theme = highlight::detect_theme();
        let lang = highlight::detect(path);
        let highlighted = lang.map(|l| highlight::highlight_file(&text, l, theme));
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
            cursor_line: 0,
            cursor_col: 0,
            lang,
            highlighted,
            theme,
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
            cursor_line: 0,
            cursor_col: 0,
            lang: None,
            highlighted: None,
            theme: crate::highlight::Theme::Dark,
        }
    }
    #[test]
    fn scroll_follows_cursor() {
        // ensure_cursor_visible keeps the cursor's display row in view.
        let mut a = viewer_app(10);
        a.ensure_cursor_visible(9, 5);
        assert_eq!(a.scroll, 5);
        a.ensure_cursor_visible(0, 5);
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
    #[test]
    fn cursor_clamps_to_text() {
        let mut a = viewer_app(3);
        a.move_cursor_line(100);
        assert_eq!(a.cursor_line, 2);
        a.move_cursor_line(-100);
        assert_eq!(a.cursor_line, 0);
        // "line 0" is 6 chars; col clamps to 6.
        a.move_cursor_col(100);
        assert_eq!(a.cursor_col, 6);
        a.move_cursor_col(-100);
        assert_eq!(a.cursor_col, 0);
    }
    #[test]
    fn load_file_detects_language() {
        let dir = std::env::temp_dir();
        let path = dir.join("ctx_hl_test.rs");
        std::fs::write(&path, "fn main() {}\n").unwrap();
        let app = App::load_file(&path, false).unwrap();
        assert_eq!(app.lang, Some(crate::highlight::Lang::Rust));
        assert!(app.highlighted.is_some());
        let _ = std::fs::remove_file(&path);
    }
}
