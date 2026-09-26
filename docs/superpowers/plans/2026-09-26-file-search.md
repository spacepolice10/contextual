# File Search with Viewport-Aware Jump Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add vim-style `/` + `n`/`N` in-file search with viewport-aware scroll (2-line margin) to the viewer.

**Architecture:** New pure module `src/search.rs` (matcher + scroll rule, TDD), extend `App` state in `src/app.rs`, key dispatch + status/highlight render in `src/main.rs`. No new deps; reuse `viewer::clamp_scroll` and `display_row_for_cursor`.

**Tech Stack:** Rust 2021, ratatui 0.29, crossterm 0.28, tree-sitter highlight (existing, untouched).

---

### Task 1: Pure search logic (`src/search.rs`)

**Files:**
- Create: `src/search.rs`
- Modify: `src/main.rs:1-4` (add `mod search;`)
- Test: inline `#[cfg(test)]` in `src/search.rs`

Public interface (locked for later tasks):
```rust
pub const MAX_MATCHES: usize = 10_000;
pub fn find_matches(lines: &[String], query: &str) -> Vec<(usize, usize)>;
pub fn scroll_for_match(match_row: usize, scroll: usize, vh: usize, margin: usize) -> usize;
```
- `find_matches`: literal substring, char-based `(line_idx, char_col)`, overlapping allowed, smart-case (case-insensitive unless query contains any uppercase), empty query → `[]`, capped at `MAX_MATCHES`.
- `scroll_for_match`: display-row based. `margin` saturates to `0` when `vh < 5` (i.e. `m = margin.min(vh.saturating_sub(1) / 2)` when `vh > 0`). No scroll if `match_row` in `[scroll+m, scroll+vh-m)`; above/near-top → `match_row.saturating_sub(m)`; below/near-bottom → `match_row + 1 + m - vh`; then `crate::viewer::clamp_scroll(result, usize::MAX, vh)` — caller clamps with real total afterward via existing render path (or pass total; v1 clamps with `usize::MAX` then render re-clamps).

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literal_basic_and_overlap() {
        let lines = vec!["aaa".to_string()];
        assert_eq!(find_matches(&lines, "aa"), vec![(0, 0), (0, 1)]);
    }
    #[test]
    fn empty_query_is_empty() {
        let lines = vec!["hi".to_string()];
        assert!(find_matches(&lines, "").is_empty());
    }
    #[test]
    fn smart_case_insensitive_then_sensitive() {
        let lines = vec!["Hello hello".to_string()];
        assert_eq!(find_matches(&lines, "hello"), vec![(0, 0), (0, 6)]);
        assert_eq!(find_matches(&lines, "Hello"), vec![(0, 0)]);
    }
    #[test]
    fn unicode_char_cols() {
        let lines = vec!["héllo 🌍x".to_string()];
        assert_eq!(find_matches(&lines, "🌍"), vec![(0, 6)]);
    }
    #[test]
    fn scroll_middle_untouched() {
        assert_eq!(scroll_for_match(10, 5, 20, 2), 5);
    }
    #[test]
    fn scroll_near_bottom_edge() {
        // vh=20, scroll=0, match row 18 -> 18-20+1+2 = 1
        assert_eq!(scroll_for_match(18, 0, 20, 2), 1);
    }
    #[test]
    fn scroll_outside_below() {
        assert_eq!(scroll_for_match(30, 0, 20, 2), 13);
    }
    #[test]
    fn scroll_outside_above() {
        assert_eq!(scroll_for_match(0, 10, 20, 2), 0);
    }
    #[test]
    fn scroll_tiny_viewport_saturates() {
        assert_eq!(scroll_for_match(2, 0, 3, 2), 0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test search:: -- --nocapture`
Expected: FAIL with "no function or associated item named ... / file not found" (module does not exist yet).

- [ ] **Step 3: Write minimal implementation**

```rust
use crate::viewer::clamp_scroll;

pub const MAX_MATCHES: usize = 10_000;

/// Literal smart-case scan. Returns (line_idx, char_col), overlapping allowed.
pub fn find_matches(lines: &[String], query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let insensitive = !query.chars().any(|c| c.is_uppercase());
    let q = if insensitive { query.to_lowercase() } else { query.to_string() };
    let qchars: Vec<char> = q.chars().collect();
    if qchars.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (li, line) in lines.iter().enumerate() {
        let hay = if insensitive { line.to_lowercase() } else { line.clone() };
        let h: Vec<char> = hay.chars().collect();
        if h.len() < qchars.len() {
            continue;
        }
        for s in 0..=h.len() - qchars.len() {
            if h[s..s + qchars.len()] == qchars[..] {
                out.push((li, s));
                if out.len() >= MAX_MATCHES {
                    return out;
                }
            }
        }
    }
    out
}

/// Minimal scroll with margin. All args are display rows.
pub fn scroll_for_match(match_row: usize, scroll: usize, vh: usize, margin: usize) -> usize {
    if vh == 0 {
        return scroll;
    }
    let m = margin.min(vh.saturating_sub(1) / 2);
    if match_row >= scroll + m && match_row < scroll + vh.saturating_sub(m) {
        return scroll;
    }
    let next = if match_row < scroll + m {
        match_row.saturating_sub(m)
    } else {
        match_row + 1 + m - vh
    };
    clamp_scroll(next, usize::MAX, vh)
}
```

Plus in `src/main.rs:1-4` add `mod search;` to existing mod list:
```rust
mod app;
mod highlight;
mod picker;
mod search;
mod viewer;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test search:: -- --nocapture`
Expected: PASS (all 9 tests).

- [ ] **Step 5: Commit**

```bash
git add src/search.rs src/main.rs
git commit -m "feat: pure file-search matcher and margin scroll rule"
```

### Task 2: App search state (`src/app.rs`)

**Files:**
- Modify: `src/app.rs:11-31` (struct fields), `src/app.rs:81-100` (`new_picker`), `src/app.rs:101-129` (`load_file`)
- Test: inline `src/app.rs` tests mod

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test app::tests::search_state_defaults_and_restore`
Expected: FAIL ("no field `searching` on type `App`").

- [ ] **Step 3: Write minimal implementation**

Add to `pub struct App` after `cursor_col`:
```rust
pub search_query: String,
pub search_matches: Vec<(usize, usize)>,
pub search_idx: usize,
pub searching: bool,
pub saved_cursor: Option<(usize, usize)>,
pub saved_scroll: Option<usize>,
```
Init in both constructors (`new_picker` and `load_file`):
```rust
search_query: String::new(),
search_matches: Vec::new(),
search_idx: 0,
searching: false,
saved_cursor: None,
saved_scroll: None,
```
Add methods:
```rust
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
    if let Some((l, c)) = self.saved_cursor {
        self.cursor_line = l;
        self.cursor_col = c;
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
```
Update `viewer_app` test helper in `src/app.rs:134-154` with the six new fields (same defaults as above).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test app::`
Expected: PASS (5 existing + 1 new).

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat: app search state with cancel restore"
```

### Task 3: Key dispatch — `/`, input, Enter/Esc, `n`/`N` (`src/main.rs`)

**Files:**
- Modify: `src/main.rs:71-152` (`handle`)
- Test: manual (event loop; unit via `handle` calls — no terminal needed)

- [ ] **Step 1: Write the failing test**

There is no test harness for `handle` yet; add inline `#[cfg(test)] mod handle_tests` at bottom of `src/main.rs`:
```rust
#[cfg(test)]
mod handle_tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    fn viewer() -> app::App {
        app::App::load_file(std::path::Path::new("Cargo.toml"), false).unwrap()
    }
    #[test]
    fn slash_enters_search_and_esc_restores() {
        let mut a = viewer();
        handle(&mut a, KeyCode::Char('/'), KeyModifiers::NONE).unwrap();
        assert!(a.searching);
        handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(!a.searching);
    }
    #[test]
    fn typing_runs_matcher_and_n_advances() {
        let mut a = viewer();
        handle(&mut a, KeyCode::Char('/'), KeyModifiers::NONE).unwrap();
        for c in "package".chars() {
            handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap();
        }
        assert!(!a.search_matches.is_empty());
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(!a.searching);
        let first = a.search_idx;
        handle(&mut a, KeyCode::Char('n'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.search_idx, (first + 1) % a.search_matches.len());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test handle_tests::`
Expected: FAIL (`handle` takes only known keys; `/` falls to `_ => Ok(false)`, `a.searching` false).

- [ ] **Step 3: Write minimal implementation**

In `handle()`, inside `Mode::Viewer` branch, first intercept search-input mode (before the existing `match code`):
```rust
if app.searching {
    match code {
        KeyCode::Esc => {
            app.cancel_search();
            return Ok(false);
        }
        KeyCode::Enter => {
            // commit: jump to first match at/after cursor, or keep position if none
            if !app.search_matches.is_empty() {
                let cur = (app.cursor_line, app.cursor_col);
                let pos = app.search_matches.iter().position(|&m| m >= cur).unwrap_or(0);
                app.search_idx = pos;
                let (l, c) = app.search_matches[pos];
                app.cursor_line = l;
                app.cursor_col = c;
                let row = crate::viewer::display_row_for_cursor(
                    &app.lines, 80.max(1), app.wrap, l, c,
                );
                app.scroll = crate::search::scroll_for_match(row, app.scroll, app.viewport_h.max(1), 2);
            }
            app.commit_search();
            return Ok(false);
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        KeyCode::Char(c) => {
            if !mods.contains(KeyModifiers::CONTROL) && !mods.contains(KeyModifiers::ALT) {
                app.search_query.push(c);
            }
        }
        _ => {}
    }
    // recompute after every input key (except Esc/Enter which returned)
    app.search_matches = crate::search::find_matches(&app.lines, &app.search_query);
    if !app.search_matches.is_empty() {
        // live preview: nearest match at/after original cursor
        let anchor = app.saved_cursor.unwrap_or((app.cursor_line, app.cursor_col));
        let pos = app.search_matches.iter().position(|&m| m >= anchor).unwrap_or(0);
        app.search_idx = pos;
        let (l, c) = app.search_matches[pos];
        app.cursor_line = l;
        app.cursor_col = c;
    }
    return Ok(false);
}
```
Then in the normal Viewer `match code`, add:
```rust
KeyCode::Char('/') => {
    app.start_search();
    Ok(false)
}
KeyCode::Char('n') => {
    if !app.search_matches.is_empty() {
        app.search_idx = (app.search_idx + 1) % app.search_matches.len();
        let (l, c) = app.search_matches[app.search_idx];
        app.cursor_line = l;
        app.cursor_col = c;
        // scroll rule applied fully in render (needs real text width); best-effort here:
    }
    Ok(false)
}
KeyCode::Char('N') => {
    if !app.search_matches.is_empty() {
        app.search_idx = (app.search_idx + app.search_matches.len() - 1) % app.search_matches.len();
        let (l, c) = app.search_matches[app.search_idx];
        app.cursor_line = l;
        app.cursor_col = c;
    }
    Ok(false)
}
```
Note: `n`/`N` scroll landing is finalized in render (Task 4) where true `text_w` is known; cursor move here is enough and `ensure_cursor_visible` keeps it safe. Do NOT steal `N` when `searching` (input branch returns first, so typing `N` still works).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test handle_tests::`
Expected: PASS (2 tests). Then `cargo test` full suite PASS.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: search key dispatch with n/N navigation"
```

### Task 4: Render — status bar + match highlights + true scroll landing (`src/main.rs`)

**Files:**
- Modify: `src/main.rs:192-349` (`render` Viewer branch)
- Test: `cargo test` + manual open of `Cargo.toml`

- [ ] **Step 1: Write the failing test**

Unit-test the scroll landing with real width via existing pure fns (no terminal):
```rust
#[test]
fn scroll_landing_uses_display_row() {
    let lines = vec!["abcdef".to_string(), "xy".to_string()];
    // width 2: line 0 -> 3 display rows; match (0,3) is display row 1
    let row = crate::viewer::display_row_for_cursor(&lines, 2, true, 0, 3);
    assert_eq!(row, 1);
    assert_eq!(crate::search::scroll_for_match(row, 0, 5, 2), 0); // middle, untouched
}
```
Add to `handle_tests` mod. Run: `cargo test handle_tests::scroll_landing_uses_display_row` → FAIL (test does not exist yet — file unchanged).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test handle_tests::scroll_landing_uses_display_row`
Expected: FAIL ("no test ... in its namespace" / compile ok but 0 tests matched — treat as red; alternatively `cargo test` shows 0 new tests).

- [ ] **Step 3: Write minimal implementation**

a) After `let cursor_row = ...` in render (`src/main.rs:212`), when a committed/last search exists and mode is Viewer, enforce the margin rule with the TRUE `text_w` (fixes the best-effort scroll from Task 3 and covers `n`/`N`):
```rust
if !app.search_matches.is_empty() {
    let (ml, mc) = app.search_matches[app.search_idx.min(app.search_matches.len() - 1)];
    // cursor already on match; recompute row with real width and apply margin rule
    let mrow = viewer::display_row_for_cursor(&app.lines, text_w.max(1), app.wrap, ml, mc);
    let m = 2usize.min(app.viewport_h.saturating_sub(1) / 2);
    let in_middle = mrow >= app.scroll + m && mrow < app.scroll + vh.saturating_sub(m);
    if !in_middle {
        app.scroll = crate::search::scroll_for_match(mrow, app.scroll, vh, 2);
        app.scroll = viewer::clamp_scroll(app.scroll, total, vh);
    }
}
```
Place AFTER the existing `ensure_cursor_visible` + clamp block so the margin rule wins over plain visibility.

b) Status bar (`src/main.rs:331`): when `app.searching`, replace first status line with:
```rust
let s = if app.searching {
    if app.search_matches.is_empty() {
        format!("/{ }  [no matches] ", app.search_query)
    } else {
        format!("/{ }  {}/{} ", app.search_query, app.search_idx + 1, app.search_matches.len())
    }
} else if !app.search_query.is_empty() && !app.search_matches.is_empty() {
    format!(
        " {}/{}  Ln {},Col {}  wrap:{}  /{} ",
        app.scroll + 1, total.max(1), app.cursor_line + 1, app.cursor_col + 1,
        if app.wrap { "ON" } else { "OFF" }, app.search_query
    )
} else {
    /* existing `s` */
};
```
Keep `status_note` / `[no highlight]` appends unchanged.

c) Highlights: in the display-row loop, build a `HashSet<usize>` of display rows containing any match once per frame (map each `(line,col)` via `display_row_for_cursor`), then style non-current match rows with underline and current match cell with reverse. Minimal approach reusing `cursor_style` machinery: for rows in the set that are NOT the cursor row, wrap body spans with `.underline()`. Keep tree-sitter spans intact (only add style, never replace text).

- [ ] **Step 4: Run tests and verify**

Run: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
Expected: fmt clean, clippy clean, all tests PASS.
Manual: `cargo run -- Cargo.toml`, press `/`, type `pack`, confirm live jump + `3/…` count, `Enter`, `n`/`N` cycle, `Esc` in new search restores; match near bottom edge scrolls with 2-line margin, middle match does not scroll.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: search status, highlights, and margin scroll landing"
```

---

## Self-Review

- Spec coverage: vim `/`+Enter+Esc+`n`/`N` ✓ (Tasks 3–4); literal smart-case ✓ (Task 1); 2-line margin minimal scroll in display rows ✓ (Tasks 1, 4); count + highlight ✓ (Task 4); empty/no-match/unicode/cap ✓ (Task 1 + 4); cancel-restore ✓ (Task 2).
- Placeholders: none — all steps have exact paths, code, commands, expected output.
- Type consistency: `Vec<(usize, usize)>` matcher output reused as `search_matches` in Tasks 2–4; `scroll_for_match(match_row, scroll, vh, margin)` signature identical everywhere; `App` field names identical across tasks.
