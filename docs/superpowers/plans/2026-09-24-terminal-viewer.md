# Terminal Text Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build minimal robust Rust TUI that opens a text file or picks one from cwd, renders with line numbers and scrolling.

**Architecture:** Binary crate `contextual` with 4 focused modules: `main` (CLI + terminal guard + loop), `app` (Mode + state + key dispatch), `picker` (dir scan pure logic), `viewer` (wrap/scroll pure logic + ratatui render). Pure logic separated for unit testing without terminal.

**Tech Stack:** Rust 2021, ratatui 0.29, crossterm 0.28, clap 4.5 (derive), anyhow 1.0

---

## File Structure

- Create: `Cargo.toml` — package `contextual`, deps above.
- Create: `src/main.rs` — clap CLI (`Option<PathBuf>`), `ensure_raw()` guard struct with `Drop`, `run_app()` event loop, `load_text_file()` with lossy UTF-8.
- Create: `src/app.rs` — `enum Mode { Picker, Viewer }`, `struct App` { mode, picker_index, files, lines, filename, scroll, h_scroll, wrap, status_note }, `handle_key()` pure dispatch.
- Create: `src/picker.rs` — `struct FileEntry { name: String, path: PathBuf, size: u64 }`, `list_files(dir: &Path) -> anyhow::Result<Vec<FileEntry>>`, `clamp_selection(idx, len)`.
- Create: `src/viewer.rs` — `wrap_line(line: &str, width: usize) -> Vec<String>`, `build_display_lines(lines: &[String], width: usize, wrap: bool) -> Vec<(usize, String)>`, `clamp_scroll(scroll, total, viewport)`, `clamp_hscroll()`.
- Test: unit tests inline in `picker.rs`, `viewer.rs`, `app.rs` via `#[cfg(test)]` (run with `cargo test`).

---

### Task 1: Scaffold crate and dependencies

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "contextual"
version = "0.1.0"
edition = "2021"
description = "Minimal terminal text viewer"
license = "MIT"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
clap = { version = "4.5", features = ["derive"] }
anyhow = "1.0"
```

- [ ] **Step 2: Create minimal src/main.rs**

```rust
fn main() {
    println!("contextual ok");
}
```

- [ ] **Step 3: Run build to verify deps resolve**

Run: `cargo build`
Expected: PASS, binary builds (warnings ok).

- [ ] **Step 4: Run binary**

Run: `./target/debug/contextual`
Expected: prints `contextual ok`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "feat: scaffold contextual binary crate"
```

---

### Task 2: Viewer pure logic (wrap + scroll) with TDD

**Files:**
- Create: `src/viewer.rs`
- Modify: `src/main.rs:1-3` (add `mod viewer;`)

- [ ] **Step 1: Write the failing tests**

```rust
// src/viewer.rs
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrap_short_line_unchanged() {
        assert_eq!(wrap_line("hi", 10), vec!["hi".to_string()]);
    }
    #[test]
    fn wrap_long_line_splits_on_chars() {
        assert_eq!(wrap_line("abcdef", 2), vec!["ab".to_string(), "cd".to_string(), "ef".to_string()]);
    }
    #[test]
    fn wrap_zero_width_returns_whole() {
        assert_eq!(wrap_line("abc", 0), vec!["abc".to_string()]);
    }
    #[test]
    fn clamp_scroll_basic() {
        assert_eq!(clamp_scroll(0, 10, 5), 0);
        assert_eq!(clamp_scroll(100, 10, 5), 5);
        assert_eq!(clamp_scroll(3, 3, 5), 0);
        assert_eq!(clamp_scroll(0, 0, 5), 0);
    }
    #[test]
    fn build_display_unwrapped_keeps_logical_index() {
        let lines = vec!["a".to_string(), "b".to_string()];
        let d = build_display_lines(&lines, 10, false);
        assert_eq!(d, vec![(0, "a".to_string()), (1, "b".to_string())]);
    }
    #[test]
    fn build_display_wrapped_expands() {
        let lines = vec!["abcdef".to_string()];
        let d = build_display_lines(&lines, 2, true);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0], (0, "ab".to_string()));
    }
}
```

Run: `cargo test viewer -- --nocapture`
Expected: FAIL with `cannot find function wrap_line` / module missing.

- [ ] **Step 2: Write minimal implementation**

```rust
/// Split one logical line into display chunks of at most `width` chars.
/// Uses char-boundary safe chunking. width==0 returns whole line.
pub fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![line.to_string()];
    }
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return vec![String::new()];
    }
    chars
        .chunks(width)
        .map(|c| c.iter().collect())
        .collect()
}

/// Build (logical_index, display_text) rows.
/// wrap=true expands long lines via wrap_line, else 1:1.
pub fn build_display_lines(lines: &[String], width: usize, wrap: bool) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let w = width.max(1);
    for (i, line) in lines.iter().enumerate() {
        if wrap {
            for chunk in wrap_line(line, w) {
                out.push((i, chunk));
            }
        } else {
            out.push((i, line.clone()));
        }
    }
    out
}

/// Clamp vertical scroll so viewport fits. Returns max valid top.
pub fn clamp_scroll(scroll: usize, total: usize, viewport: usize) -> usize {
    if total <= viewport || viewport == 0 {
        return 0;
    }
    scroll.min(total - viewport)
}

/// Clamp horizontal scroll (used when wrap is off).
pub fn clamp_hscroll(h: usize, max_width: usize, viewport_w: usize) -> usize {
    if max_width <= viewport_w || viewport_w == 0 {
        return 0;
    }
    h.min(max_width - viewport_w)
}
```

Also add to `src/main.rs` top: `mod viewer;`

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test viewer`
Expected: PASS, 6 tests ok.

- [ ] **Step 4: Commit**

```bash
git add src/viewer.rs src/main.rs
git commit -m "feat: add viewer wrap and scroll logic with tests"
```

---

### Task 3: Picker logic (list + clamp) with TDD

**Files:**
- Create: `src/picker.rs`
- Modify: `src/main.rs` (add `mod picker;`)

- [ ] **Step 1: Write the failing tests**

```rust
// src/picker.rs tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn clamp_selection_empty_is_zero() {
        assert_eq!(clamp_selection(5, 0), 0);
    }
    #[test]
    fn clamp_selection_bounds() {
        assert_eq!(clamp_selection(0, 3), 0);
        assert_eq!(clamp_selection(99, 3), 2);
        assert_eq!(clamp_selection(1, 3), 1);
    }
    #[test]
    fn list_files_only_regular_sorted() {
        let dir = std::env::temp_dir().join(format!("ctx_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("b.txt"), "b").unwrap();
        fs::write(dir.join("a.txt"), "a").unwrap();
        let files = list_files(&dir).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.name.clone()).collect();
        assert_eq!(names, vec!["a.txt".to_string(), "b.txt".to_string()]);
        let _ = fs::remove_dir_all(&dir);
    }
}
```

Run: `cargo test picker`
Expected: FAIL, `cannot find function list_files`.

- [ ] **Step 2: Write minimal implementation**

```rust
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
}

/// List regular files in `dir`, sorted by name. Skips dirs/symlink-dirs/errors.
pub fn list_files(dir: &Path) -> Result<Vec<FileEntry>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(FileEntry { name, path, size: meta.len() });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Clamp selection index into 0..len.
pub fn clamp_selection(idx: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    idx.min(len - 1)
}
```

Add `mod picker;` to `src/main.rs`.

- [ ] **Step 3: Run tests**

Run: `cargo test picker`
Expected: PASS, 3 tests ok.

- [ ] **Step 4: Commit**

```bash
git add src/picker.rs src/main.rs
git commit -m "feat: add picker file listing with tests"
```

---

### Task 4: App state and key dispatch

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs` (add `mod app;`)

- [ ] **Step 1: Write the failing test**

```rust
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
```

Run: `cargo test app`
Expected: FAIL, struct/methods missing.

- [ ] **Step 2: Write minimal implementation**

```rust
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
        let clamped = new.clamp(0, i64::MAX) as usize;
        self.scroll = clamp_scroll(clamped, total_hint, viewport_h);
    }
    pub fn toggle_wrap(&mut self) {
        self.wrap = !self.wrap;
        self.h_scroll = 0;
    }
    pub fn move_picker(&mut self, delta: isize) {
        let n = self.picker_index as isize + delta;
        let n = n.clamp(0, i64::MAX) as usize;
        self.picker_index = clamp_selection(n, self.files.len());
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test app`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: add app state with scroll and wrap"
```

---

### Task 5: Terminal wiring, rendering, event loop, CLI

**Files:**
- Modify: `src/main.rs` (full rewrite below)
- Modify: `src/app.rs` (add constructors + load helpers — full additions below)

- [ ] **Step 1: Extend app.rs with constructors**

```rust
// Append to src/app.rs
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
        }
    }
    pub fn load_file(path: &Path, from_picker: bool) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("cannot open {}", path.display()))?;
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
        })
    }
}
```

- [ ] **Step 2: Write full main.rs**

```rust
mod app;
mod picker;
mod viewer;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction, Layout}, style::{Color, Style}, text::{Line, Span}, widgets::{Block, Borders, List, ListItem, Paragraph}, Terminal};
use std::{io, path::PathBuf, time::Duration};

#[derive(Parser, Debug)]
#[command(name = "contextual", about = "Minimal terminal text viewer")]
struct Cli {
    /// File to open. If omitted, shows picker for current directory.
    path: Option<PathBuf>,
}

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> Result<Self> {
        crossterm::terminal::enable_raw_mode().context("enable raw mode")?;
        let mut out = io::stdout();
        crossterm::execute!(out, crossterm::terminal::EnterAlternateScreen).context("enter alt screen")?;
        Ok(Self)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut app = match cli.path {
        Some(p) => app::App::load_file(&p, false)?,
        None => {
            let files = picker::list_files(std::path::Path::new("."))?;
            app::App::new_picker(files)
        }
    };
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut term = Terminal::new(backend).context("create terminal")?;
    loop {
        term.draw(|f| render(f, &mut app))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                if handle(&mut app, k.code, k.modifiers)? {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn handle(app: &mut app::App, code: KeyCode, mods: KeyModifiers) -> Result<bool> {
    use app::Mode;
    if mods.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return Ok(true);
    }
    match app.mode {
        Mode::Picker => match code {
            KeyCode::Char('q') | KeyCode::Esc => Ok(true),
            KeyCode::Down | KeyCode::Char('j') => { app.move_picker(1); Ok(false) }
            KeyCode::Up | KeyCode::Char('k') => { app.move_picker(-1); Ok(false) }
            KeyCode::Enter => {
                if let Some(f) = app.files.get(app.picker_index).cloned() {
                    *app = app::App::load_file(&f.path, true)?;
                }
                Ok(false)
            }
            _ => Ok(false),
        },
        Mode::Viewer => match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if app.from_picker && code == KeyCode::Esc {
                    let files = picker::list_files(std::path::Path::new("."))?;
                    *app = app::App::new_picker(files);
                    Ok(false)
                } else { Ok(true) }
            }
            KeyCode::Down | KeyCode::Char('j') => { app.scroll_by(1, 20); Ok(false) }
            KeyCode::Up | KeyCode::Char('k') => { app.scroll_by(-1, 20); Ok(false) }
            KeyCode::PageDown => { app.scroll_by(20, 20); Ok(false) }
            KeyCode::PageUp => { app.scroll_by(-20, 20); Ok(false) }
            KeyCode::Home | KeyCode::Char('g') => { app.scroll = 0; Ok(false) }
            KeyCode::End | KeyCode::Char('G') => { app.scroll = usize::MAX; app.scroll_by(0, 20); Ok(false) }
            KeyCode::Char('w') => { app.toggle_wrap(); Ok(false) }
            KeyCode::Left => { app.h_scroll = app.h_scroll.saturating_sub(4); Ok(false) }
            KeyCode::Right => { app.h_scroll += 4; Ok(false) }
            _ => Ok(false),
        },
    }
}

fn render(f: &mut ratatui::Frame, app: &mut app::App) {
    use app::Mode;
    let area = f.area();
    match app.mode {
        Mode::Picker => {
            let items: Vec<ListItem> = app.files.iter().map(|e| {
                ListItem::new(Line::from(vec![Span::raw(format!("{}  ({}b)", e.name, e.size))]))
            }).collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" contextual — pick a file (Enter/q) "))
                .highlight_style(Style::default().bg(Color::DarkGray));
            if app.files.is_empty() {
                let p = Paragraph::new("No files in this directory — press q to quit.")
                    .block(Block::default().borders(Borders::ALL).title(" contextual "));
                f.render_widget(p, area);
            } else {
                use ratatui::widgets::ListState;
                let mut st = ListState::default();
                st.select(Some(app.picker_index));
                f.render_stateful_widget(list, area, &mut st);
            }
        }
        Mode::Viewer => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(2)])
                .split(area);
            let gutter = app.lines.len().to_string().len().max(4) + 1;
            let text_w = (chunks[0].width as usize).saturating_sub(gutter + 1);
            let display = viewer::build_display_lines(&app.lines, text_w.max(1), app.wrap);
            let total = display.len();
            let vh = chunks[0].height as usize;
            app.scroll = viewer::clamp_scroll(app.scroll, total, vh);
            let end = (app.scroll + vh).min(total);
            let mut text = Vec::new();
            for (drow, (lidx, content)) in display[app.scroll..end].iter().enumerate() {
                let _ = drow;
                let shown = if app.wrap {
                    content.clone()
                } else {
                    content.chars().skip(app.h_scroll).take(text_w.max(1)).collect()
                };
                text.push(Line::from(vec![
                    Span::styled(format!("{:>width$} ", lidx + 1, width = gutter - 1), Style::default().fg(Color::DarkGray)),
                    Span::raw(shown),
                ]));
            }
            let body = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title(format!(" {} ", app.filename)));
            f.render_widget(body, chunks[0]);
            let mut s = format!(" {}  {}/{}  wrap:{}  [w]rap [q]uit ", app.filename, app.scroll + 1, total.max(1), if app.wrap { "ON" } else { "OFF" });
            if let Some(n) = &app.status_note {
                s.push_str(n);
            }
            let bar = Paragraph::new(vec![
                Line::from(Span::raw(s)),
                Line::from(Span::styled(" ↑↓ scroll  PgUp/PgDn  g/G top/bottom  w wrap  ←→ h-scroll  q quit ", Style::default().fg(Color::Gray))),
            ]);
            f.render_widget(bar, chunks[1]);
        }
    }
}
```

- [ ] **Step 3: Build and fix warnings**

Run: `cargo build 2>&1 | tail -20`
Expected: builds with zero warnings (fix any).

Run: `cargo fmt && cargo clippy -- -D warnings`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/app.rs
git commit -m "feat: wire TUI event loop, rendering, and CLI"
```

---

### Task 6: Verify quality gates and manual smoke

**Files:** none (verification only)

- [ ] **Step 1: Run full checks**

Run: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
Expected: all PASS, ~10 tests green.

- [ ] **Step 2: Manual smoke (human)**

Run: `echo hello > /tmp/t.txt && ./target/debug/contextual /tmp/t.txt`
Expected: shows `hello` with line number 1, `w` toggles wrap, `q` quits and restores terminal.

Run: `./target/debug/contextual`
Expected: picker lists files in repo dir, Enter opens, Esc back, q quits.

- [ ] **Step 3: Commit any fixes**

```bash
git add -A
git commit -m "fix: address review findings" || true
```

---

## Self-Review

- Spec coverage: open+scroll ✓ (Task 2+5), line numbers+status ✓ (Task 5), open errors via anyhow+guard ✓ (Task 5), wrap toggle `w` ✓ (Task 4+5), arg-or-picker ✓ (Task 3+5), resize via ratatui recompute each frame ✓ (Task 5).
- Placeholders: none — all steps have exact paths, full code, exact commands.
- Type consistency: `wrap_line(&str, usize)->Vec<String>`, `build_display_lines(&[String], usize, bool)->Vec<(usize,String)>`, `clamp_scroll(usize,usize,usize)->usize`, `FileEntry{name,path,size}`, `App{mode,files,picker_index,lines,filename,scroll,h_scroll,wrap,status_note,from_picker}` used consistently.
