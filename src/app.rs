use crate::highlight::{self, Lang};
use crate::picker::{clamp_selection, filter_files, FileEntry, ScoredMatch};
use crate::select::Selection;
use ratatui::text::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Picker,
    Viewer,
}

/// Memory-only annotation on a text span (Task 3 fills, Task 4 lists).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub id: usize,
    pub file: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub snippet: String,
    pub note: String,
}

/// Draft comment being typed (Task 3 fills).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingComment {
    pub snippet: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub draft: String,
}

/// Fuzzy-picker input state: live query, ranked hits into `App::files`,
/// keyboard selection, and preview scroll. `filtered` holds `ScoredMatch`
/// values whose `entry_idx` points at `App::files`.
/// Staged helper (render/keys land in Tasks 4-5); allow dead code until then.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct PickerState {
    pub query: String,
    pub filtered: Vec<ScoredMatch>,
    pub selected: usize,
    pub preview_scroll: usize,
    pub truncated: bool,
}

#[derive(Debug)]
pub struct App {
    pub mode: Mode,
    pub files: Vec<FileEntry>,
    pub picker_index: usize,
    /// Fuzzy state shadowing `files`/`picker_index` (migration lands in Task 6).
    pub picker: PickerState,
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
    pub search_query: String,
    pub search_matches: Vec<(usize, usize)>,
    pub search_idx: usize,
    pub searching: bool,
    pub saved_cursor: Option<(usize, usize)>,
    pub saved_scroll: Option<usize>,
    pub lang: Option<Lang>,
    pub highlighted: Option<Vec<Vec<Span<'static>>>>,
    // Theme used at load for `highlighted`; reserved for future re-highlight.
    #[allow(dead_code)]
    pub theme: highlight::Theme,
    /// Active visual selection; anchor end fixed, cursor is the live end.
    pub visual: Option<Selection>,
    /// Accumulated count prefix (`5j`, `v3w`); cleared after a motion.
    pub pending_count: Option<usize>,
    /// Pending `i`/`a` text-object in visual (`Some(true)` = inner `i`,
    /// `Some(false)` = around `a`); consumed by the next delimiter key.
    pub pending_object: Option<bool>,
    /// Draft comment input (Task 3).
    #[allow(dead_code)]
    pub commenting: Option<PendingComment>,
    /// Saved comments, memory-only (Task 3 fills, Task 4 lists).
    #[allow(dead_code)]
    pub comments: Vec<Comment>,
    /// Sidebar visibility (Task 4 renders).
    #[allow(dead_code)]
    pub show_sidebar: bool,
}

impl App {
    pub fn toggle_wrap(&mut self) {
        self.wrap = !self.wrap;
        self.h_scroll = 0;
    }
    /// Legacy index mover (render still reads `picker_index`; Task 5/6
    /// migrate render to `picker.selected`). Allow dead code until then.
    #[allow(dead_code)]
    pub fn move_picker(&mut self, delta: isize) {
        let n = self.picker_index as isize + delta;
        let n = n.clamp(0, isize::MAX) as usize;
        self.picker_index = clamp_selection(n, self.files.len());
    }
    /// Re-rank `files` against the picker query. Keeps `selected` when it
    /// still points inside the new list, else clamps to 0. Resets the
    /// preview scroll when the selected file changes.
    /// Staged helper (keys land in Task 4); allow dead code until then.
    #[allow(dead_code)]
    pub fn picker_recompute(&mut self) {
        let before = self.picker_selected_entry();
        self.picker.filtered = filter_files(&self.files, &self.picker.query);
        if self.picker.selected >= self.picker.filtered.len() {
            self.picker.selected = 0;
        }
        if self.picker_selected_entry() != before {
            self.picker.preview_scroll = 0;
        }
    }
    /// Move the fuzzy selection by `delta`, wrapping around. No-op when empty.
    /// Staged helper (keys land in Task 4); allow dead code until then.
    #[allow(dead_code)]
    pub fn picker_move(&mut self, delta: isize) {
        let len = self.picker.filtered.len();
        if len == 0 {
            self.picker.selected = 0;
            return;
        }
        let next = (self.picker.selected as isize + delta).rem_euclid(len as isize);
        self.picker.selected = next as usize;
        self.picker.preview_scroll = 0;
    }
    /// Clear the query and restore the full list at selection 0.
    /// Staged helper (keys land in Task 4); allow dead code until then.
    #[allow(dead_code)]
    pub fn picker_clear(&mut self) {
        self.picker.query.clear();
        self.picker_recompute();
        self.picker.selected = 0;
    }
    fn picker_selected_entry(&self) -> Option<usize> {
        self.picker
            .filtered
            .get(self.picker.selected)
            .map(|m| m.entry_idx)
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
    /// Char class for word motions: 0 = whitespace/EOL, 1 = word
    /// (alphanumeric + `_`), 2 = punctuation. `big` collapses to
    /// WORD semantics (whitespace vs non-whitespace).
    fn word_class(c: char, big: bool) -> u8 {
        if c.is_whitespace() {
            0
        } else if big || c.is_alphanumeric() || c == '_' {
            1
        } else {
            2
        }
    }
    fn class_at(&self, line: usize, col: usize, big: bool) -> u8 {
        self.lines
            .get(line)
            .and_then(|l| l.chars().nth(col))
            .map(|c| Self::word_class(c, big))
            .unwrap_or(0)
    }
    fn line_len(&self, line: usize) -> usize {
        self.lines.get(line).map(|l| l.chars().count()).unwrap_or(0)
    }
    /// Next char position (EOL counts as a whitespace step onto the next
    /// line); `None` at end of file.
    fn next_pos(&self, line: usize, col: usize) -> Option<(usize, usize)> {
        if line >= self.lines.len() {
            return None;
        }
        if col < self.line_len(line) {
            Some((line, col + 1))
        } else if line + 1 < self.lines.len() {
            Some((line + 1, 0))
        } else {
            None
        }
    }
    fn prev_pos(&self, line: usize, col: usize) -> Option<(usize, usize)> {
        if line >= self.lines.len() {
            return None;
        }
        if col > 0 {
            Some((line, col - 1))
        } else if line > 0 {
            Some((line - 1, self.line_len(line - 1)))
        } else {
            None
        }
    }
    fn set_cursor(&mut self, line: usize, col: usize) {
        if self.lines.is_empty() {
            self.cursor_line = 0;
            self.cursor_col = 0;
            return;
        }
        self.cursor_line = line.min(self.lines.len() - 1);
        self.cursor_col = col.min(self.line_len(self.cursor_line));
    }
    /// Vim `w` / `W`: next word/WORD start (crosses lines).
    pub fn move_word_forward(&mut self, big: bool) {
        if self.lines.is_empty() {
            return;
        }
        let mut line = self.cursor_line.min(self.lines.len() - 1);
        let mut col = self.cursor_col.min(self.line_len(line));
        let cur = self.class_at(line, col, big);
        let mut pos = match self.next_pos(line, col) {
            Some(p) => p,
            None => return,
        };
        if cur != 0 && self.class_at(pos.0, pos.1, big) == cur {
            while let Some(n) = self.next_pos(pos.0, pos.1) {
                if self.class_at(n.0, n.1, big) != cur {
                    pos = n;
                    break;
                }
                pos = n;
            }
        }
        while self.class_at(pos.0, pos.1, big) == 0 {
            match self.next_pos(pos.0, pos.1) {
                Some(n) => pos = n,
                None => return,
            }
        }
        (line, col) = pos;
        self.set_cursor(line, col);
    }
    /// Vim `e` / `E`: end of current/next word/WORD (crosses lines).
    pub fn move_word_end(&mut self, big: bool) {
        if self.lines.is_empty() {
            return;
        }
        let line = self.cursor_line.min(self.lines.len() - 1);
        let col = self.cursor_col.min(self.line_len(line));
        let mut pos = match self.next_pos(line, col) {
            Some(p) => p,
            None => return,
        };
        while self.class_at(pos.0, pos.1, big) == 0 {
            match self.next_pos(pos.0, pos.1) {
                Some(n) => pos = n,
                None => return,
            }
        }
        let cur = self.class_at(pos.0, pos.1, big);
        while let Some(n) = self.next_pos(pos.0, pos.1) {
            if self.class_at(n.0, n.1, big) != cur {
                break;
            }
            pos = n;
        }
        self.set_cursor(pos.0, pos.1);
    }
    /// Vim `b` / `B`: previous word/WORD start (crosses lines).
    pub fn move_word_back(&mut self, big: bool) {
        if self.lines.is_empty() {
            return;
        }
        let line = self.cursor_line.min(self.lines.len() - 1);
        let col = self.cursor_col.min(self.line_len(line));
        let mut pos = match self.prev_pos(line, col) {
            Some(p) => p,
            None => return,
        };
        while self.class_at(pos.0, pos.1, big) == 0 {
            match self.prev_pos(pos.0, pos.1) {
                Some(n) => pos = n,
                None => {
                    self.set_cursor(0, 0);
                    return;
                }
            }
        }
        let cur = self.class_at(pos.0, pos.1, big);
        while let Some(n) = self.prev_pos(pos.0, pos.1) {
            if self.class_at(n.0, n.1, big) != cur {
                break;
            }
            pos = n;
        }
        self.set_cursor(pos.0, pos.1);
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
    pub fn start_search(&mut self) {
        self.searching = true;
        self.saved_cursor = Some((self.cursor_line, self.cursor_col));
        self.saved_scroll = Some(self.scroll);
        self.search_query.clear();
        self.search_matches.clear();
        self.search_idx = 0;
    }
    pub fn cancel_search(&mut self) {
        self.searching = false;
        if let Some((line, col)) = self.saved_cursor {
            self.cursor_line = line;
            self.cursor_col = col;
        }
        if let Some(s) = self.saved_scroll {
            self.scroll = s;
        }
        self.search_query.clear();
        self.search_matches.clear();
        self.saved_cursor = None;
        self.saved_scroll = None;
    }
    pub fn commit_search(&mut self) {
        self.searching = false;
        self.saved_cursor = None;
        self.saved_scroll = None;
    }
}

use anyhow::{Context, Result};
use std::path::Path;

impl App {
    pub fn new_picker(files: Vec<FileEntry>) -> Self {
        let filtered = filter_files(&files, "");
        Self {
            mode: Mode::Picker,
            files,
            picker_index: 0,
            picker: PickerState {
                filtered,
                ..PickerState::default()
            },
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
            search_query: String::new(),
            search_matches: Vec::new(),
            search_idx: 0,
            searching: false,
            saved_cursor: None,
            saved_scroll: None,
            lang: None,
            highlighted: None,
            theme: highlight::Theme::Dark,
            visual: None,
            pending_count: None,
            pending_object: None,
            commenting: None,
            comments: Vec::new(),
            show_sidebar: false,
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
        let highlighted = lang.and_then(|l| highlight::highlight_file(&text, l, theme));
        Ok(Self {
            mode: Mode::Viewer,
            files: vec![],
            picker_index: 0,
            picker: PickerState::default(),
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
            search_query: String::new(),
            search_matches: Vec::new(),
            search_idx: 0,
            searching: false,
            saved_cursor: None,
            saved_scroll: None,
            lang,
            highlighted,
            theme,
            visual: None,
            pending_count: None,
            pending_object: None,
            commenting: None,
            comments: Vec::new(),
            show_sidebar: false,
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
            picker: PickerState::default(),
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
            search_query: String::new(),
            search_matches: Vec::new(),
            search_idx: 0,
            searching: false,
            saved_cursor: None,
            saved_scroll: None,
            lang: None,
            highlighted: None,
            theme: crate::highlight::Theme::Dark,
            visual: None,
            pending_count: None,
            pending_object: None,
            commenting: None,
            comments: Vec::new(),
            show_sidebar: false,
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
    #[test]
    fn search_state_defaults_and_restore() {
        let mut a = viewer_app(3);
        assert!(!a.searching);
        assert!(a.search_query.is_empty());
        a.searching = true;
        a.saved_cursor = Some((1, 2));
        a.saved_scroll = Some(4);
        a.cancel_search();
        assert!(!a.searching);
        assert_eq!((a.cursor_line, a.cursor_col), (1, 2));
        assert_eq!(a.scroll, 4);
    }
    fn word_app(lines: &[&str]) -> App {
        let mut a = viewer_app(0);
        a.lines = lines.iter().map(|s| s.to_string()).collect();
        a
    }
    fn picker_entry(path: &str) -> FileEntry {
        FileEntry {
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            path: std::path::PathBuf::from(path),
            size: 0,
        }
    }
    fn picker_app() -> App {
        App::new_picker(vec![
            picker_entry("docs/notes.md"),
            picker_entry("src/main.rs"),
        ])
    }
    #[test]
    fn picker_recompute_filters_and_clamps_selection() {
        let mut a = picker_app();
        assert_eq!(a.picker.filtered.len(), 2);
        a.picker.query = "main".to_string();
        a.picker.selected = 99;
        a.picker_recompute();
        assert_eq!(a.picker.filtered.len(), 1);
        assert_eq!(a.picker.selected, 0);
    }
    #[test]
    fn picker_move_wraps_around() {
        let mut a = picker_app();
        a.picker_move(1);
        assert_eq!(a.picker.selected, 1);
        a.picker_move(1);
        assert_eq!(a.picker.selected, 0);
        a.picker_move(-1);
        assert_eq!(a.picker.selected, 1);
        // Empty list: no-op, stays 0.
        a.picker.query = "zzz-no-match".to_string();
        a.picker_recompute();
        assert!(a.picker.filtered.is_empty());
        a.picker_move(1);
        assert_eq!(a.picker.selected, 0);
    }
    #[test]
    fn picker_clear_resets_query_and_selection() {
        let mut a = picker_app();
        a.picker.query = "main".to_string();
        a.picker_recompute();
        a.picker_move(0);
        a.picker_clear();
        assert!(a.picker.query.is_empty());
        assert_eq!(a.picker.selected, 0);
        assert_eq!(a.picker.filtered.len(), 2);
    }
    #[test]
    fn word_forward_basic() {
        let mut a = word_app(&["foo bar"]);
        a.move_word_forward(false);
        assert_eq!((a.cursor_line, a.cursor_col), (0, 4));
    }
    #[test]
    fn word_forward_stops_at_punctuation() {
        let mut a = word_app(&["foo,bar"]);
        a.move_word_forward(false);
        assert_eq!((a.cursor_line, a.cursor_col), (0, 3));
        a.move_word_forward(false);
        assert_eq!((a.cursor_line, a.cursor_col), (0, 4));
    }
    #[test]
    fn big_word_skips_punctuation_run() {
        let mut a = word_app(&["foo,bar baz"]);
        a.move_word_forward(true);
        assert_eq!((a.cursor_line, a.cursor_col), (0, 8));
    }
    #[test]
    fn word_end_basic() {
        let mut a = word_app(&["foo bar"]);
        a.move_word_end(false);
        assert_eq!((a.cursor_line, a.cursor_col), (0, 2));
        a.move_word_end(false);
        assert_eq!((a.cursor_line, a.cursor_col), (0, 6));
    }
    #[test]
    fn word_back_basic() {
        let mut a = word_app(&["foo bar"]);
        a.cursor_line = 0;
        a.cursor_col = 4;
        a.move_word_back(false);
        assert_eq!((a.cursor_line, a.cursor_col), (0, 0));
    }
    #[test]
    fn word_forward_crosses_lines() {
        let mut a = word_app(&["foo", "bar"]);
        a.cursor_line = 0;
        a.cursor_col = 2;
        a.move_word_forward(false);
        assert_eq!((a.cursor_line, a.cursor_col), (1, 0));
    }
    #[test]
    fn word_back_crosses_lines() {
        let mut a = word_app(&["foo", "bar"]);
        a.cursor_line = 1;
        a.cursor_col = 0;
        a.move_word_back(false);
        assert_eq!((a.cursor_line, a.cursor_col), (0, 0));
    }
}
