# Visual Select + Inline Comments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add vim-style visual selection with counts and delimiter objects, Enter-to-comment into a memory-only buffer, and a toggleable sidebar to the terminal viewer.

**Architecture:** New pure module `src/select.rs` holds range math; `App` gains `visual`, `pending_count`, `commenting`, `comments`, `show_sidebar`; `main.rs::handle` wires counts/visual/objects/comment flow and `render` paints selection plus sidebar plus input row. No disk writes, no clipboard.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28, char-based indexing (`chars().count()`), inline `#[cfg(test)]` modules.

**Spec:** `docs/superpowers/specs/2026-09-27-visual-select-comments-design.md`

## Global Constraints

- All text indexing is char-based (`chars().count()`), never byte-slice except guarded tree-sitter ranges.
- Keep `cargo clippy -- -D warnings` clean.
- `cargo test` must pass after every task.
- No new dependencies.
- No persistence, no sidebar focus/scroll, no yank/clipboard, no block-wise select (deferred per spec).

## Review Focus

- Wrapped long line with selection spanning 3+ chunks: only visible chunks painted, cursor cell still wins.
- Unwrapped line with `h_scroll > 0`: selection clipped to `[h_scroll, h_scroll+w)` window.
- `0` with pending count (`10`) vs bare `0` (col 0): bare goes to col 0, `10j` moves 10 lines.
- Delimiter miss (`vi"` with no quote): keeps selection, flashes `no match`, no panic.
- Empty file or empty draft comment: `v`/`Enter` noop on empty file; empty note allowed and saved.

---

## File structure

- Create: `src/select.rs` — `SelectKind::{Char,Line}`, `Selection{anchor:(usize,usize),kind}`, `normalize()`, `extract_text()`, `find_delim_pair()`, `selection_chunks()`. Pure, no ratatui.
- Modify: `src/app.rs:12-37` — add `visual: Option<Selection>`, `pending_count: Option<usize>`, `commenting: Option<PendingComment>`, `comments: Vec<Comment>`, `show_sidebar: bool`; types `SelectKind`, `Selection`, `Comment{id,file,start,end,snippet,note}`, `PendingComment{snippet,start,end,draft}`.
- Modify: `src/main.rs:72-306` (`handle`) — count accumulator, `v`/`V`/`Esc`, count-applied motions, `i`/`a` object pending, `Enter` comment start/save, `C` toggle.
- Modify: `src/main.rs:308-620` (`render`, `paint_search_ranges`, `status_line`, `viewer_hint`) — blue selection paint, sidebar split, comment input row, mode tags.
- Test: inline modules in `src/select.rs`, `src/app.rs`, `src/main.rs::handle_tests`.

Each task below is independently runnable and committable. Do not start next task in the same run.

---

### Task 1: Selection math core

**Files:**
- Create: `src/select.rs`
- Test: inline `src/select.rs` tests

**Interfaces:**
- Consumes: nothing (pure; `lines: &[String]` logical lines, char cols).
- Produces: `pub enum SelectKind { Char, Line }`, `pub struct Selection { pub anchor: (usize, usize), pub kind: SelectKind }`, `pub fn normalize(anchor: (usize,usize), cursor: (usize,usize), kind: SelectKind) -> ((usize,usize),(usize,usize))`, `pub fn extract_text(lines: &[String], start: (usize,usize), end: (usize,usize), kind: SelectKind) -> String`, `pub fn find_delim_pair(lines: &[String], cursor: (usize,usize), open: char, close: char, inner: bool) -> Option<((usize,usize),(usize,usize))>`, `pub fn selection_chunks(start: (usize,usize), end: (usize,usize), line: usize, coff: usize, nchars: usize) -> Vec<(usize,usize)>` for later paint task.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn normalize_swaps_anchor_cursor() {
    assert_eq!(normalize((3, 5), (1, 2), SelectKind::Char), ((1, 2), (3, 5)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test normalize_swaps_anchor_cursor`
Expected: FAIL with compile error (module does not exist yet)

- [ ] **Step 3: Implement `normalize`, `Selection`, `SelectKind` in `src/select.rs` plus `mod select;` wiring**

Minimal swap + line-kind full-line expansion deferred to extract/chunks.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test normalize_swaps_anchor_cursor`
Expected: PASS

- [ ] **Step 5: Add extract + delim-pair + chunks tests (same TDD cycle)**

```rust
#[test]
fn extract_multiline_charwise() {
    let lines = vec!["hello".to_string(), "world".to_string()];
    assert_eq!(extract_text(&lines, (0, 3), (1, 2), SelectKind::Char), "lo\nwo");
}
#[test]
fn delim_inner_quotes_same_line() {
    let lines = vec!["a \"hi\" b".to_string()];
    assert_eq!(find_delim_pair(&lines, (0, 4), '"', '"', true), Some(((0, 3), (0, 5))));
}
#[test]
fn delim_miss_returns_none() {
    let lines = vec!["no quotes".to_string()];
    assert_eq!(find_delim_pair(&lines, (0, 0), '"', '"', true), None);
}
#[test]
fn chunks_clip_to_window() {
    assert_eq!(selection_chunks((0, 1), (0, 5), 0, 2, 3), vec![(0, 3)]);
}
```

Run: `cargo test select`
Expected: PASS, `cargo clippy -- -D warnings` clean. Also covers Review Focus delimiter-miss line.

- [ ] **Step 6: Commit**

```bash
git add src/select.rs src/main.rs
git commit -m "feat: add selection math core"
```

Acceptance: pure range math tested, no UI yet.

---

### Task 2: Visual state + counts + motions

**Files:**
- Modify: `src/app.rs:12-37` (add `visual`, `pending_count`, `comments: Vec<Comment>`, `commenting`, `show_sidebar` fields + types; `comments`/`commenting` unused until Task 3 but must compile)
- Modify: `src/main.rs:72-306` (count accumulator, `v`/`V`/`Esc`, count-applied `hjkl wWbBeE 0^$ gG` + half-page)
- Test: `src/main.rs::handle_tests`-style + `src/app.rs` tests

**Interfaces:**
- Consumes: `select::normalize`, `select::SelectKind`, `select::Selection` from Task 1.
- Produces: `App.visual`, `App.pending_count`, visual entry/exit + count-applied cursor moves for Task 3/4.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn visual_count_word_motion() {
    let mut a = viewer();
    a.lines = vec!["foo bar baz".to_string()];
    handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
    for c in ['3', 'w'] { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
    assert_eq!((a.cursor_line, a.cursor_col), (0, 8));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test visual_count_word_motion`
Expected: FAIL (no `visual`/`pending_count` fields)

- [ ] **Step 3: Implement `App` fields + `v`/`V` toggle + digit accumulator (`1-9` accumulate, bare `0` = col 0) + apply count to motions in visual and normal mode; `Esc` clears pending then visual**

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test visual_count_word_motion`
Expected: PASS. Also covers Review Focus `0` vs `10` line.

- [ ] **Step 5: Add `5j`-in-visual + `Esc`-exits tests, run full suite + clippy**

```rust
#[test]
fn visual_5j_and_esc() {
    let mut a = viewer();
    a.lines = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string(), "e".to_string(), "f".to_string()];
    handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
    for c in ['5', 'j'] { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
    assert_eq!(a.cursor_line, 5);
    handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
    assert!(a.visual.is_none());
}
```

Run: `cargo test && cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: add visual state with count motions"
```

Acceptance: `v`/`V` enter, counts work, `Esc` exits, anchor stays while cursor moves.

---

### Task 3: Delimiter objects + Enter-to-comment buffer

**Files:**
- Modify: `src/main.rs:72-306` (`i`/`a` + delimiter in visual, `Enter` start/save, commenting input branch, `Esc` cancel back to visual)
- Modify: `src/app.rs` (use `PendingComment`, `Comment` fields added in Task 2)
- Test: handle tests for `vi"`, miss flash, save with file ref + empty draft

**Interfaces:**
- Consumes: `select::find_delim_pair`, `select::extract_text`, `App.visual`, Task 2 motions.
- Produces: `App.commenting: Option<PendingComment>`, `App.comments: Vec<Comment>` filled for Task 4 sidebar.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn vi_quote_then_enter_saves_comment() {
    let mut a = viewer();
    a.lines = vec!["a \"hi\" b".to_string()];
    a.cursor_col = 4;
    handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
    for c in ['i', '"'] { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
    handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
    assert!(a.commenting.is_some());
    for c in "ok".chars() { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
    handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
    assert_eq!(a.comments.len(), 1);
    assert_eq!(a.comments[0].snippet, "hi");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test vi_quote_then_enter_saves_comment`
Expected: FAIL (no `i`/`a` object handling)

- [ ] **Step 3: Implement `i`/`a` pending-delimiter in visual via `find_delim_pair` (miss → `status_note="no match"`); `Enter` with visual starts `commenting{snippet: extract_text(...), span, draft: ""}`; commenting branch edits `draft`, `Enter` pushes `Comment{file: filename, span, snippet, note: draft}` and clears visual, `Esc` drops commenting back to visual; `Enter` with no visual = noop**

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test vi_quote_then_enter_saves_comment`
Expected: PASS. Covers Review Focus empty-draft variant below.

- [ ] **Step 5: Add miss + empty-file + empty-note tests, run suite + clippy**

```rust
#[test]
fn delim_miss_keeps_selection() {
    let mut a = viewer();
    a.lines = vec!["no quotes".to_string()];
    handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
    for c in ['i', '"'] { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
    assert!(a.visual.is_some());
    assert!(a.commenting.is_none());
}
```

Run: `cargo test && cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: add delimiter objects and comment save"
```

Acceptance: `vi"` selects, `Enter`→type→`Enter` saves `{file,span,snippet,note}` to memory buffer.

---

### Task 4: Selection paint + sidebar + status

**Files:**
- Modify: `src/main.rs:308-620` (selection paint pass with blue bg via `selection_chunks`, horizontal `[text | 35]` split on `show_sidebar`, comment input row, `viewer_hint` + status tags + `C` toggle)
- Test: `status_tests`-style + paint helper test

**Interfaces:**
- Consumes: `select::selection_chunks`, `select::normalize`, `App.visual/comments/commenting/show_sidebar` from Tasks 2–3.
- Produces: visible selection, sidebar list, input row. Nothing downstream.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn status_shows_visual_and_comment_count() {
    let mut a = viewer();
    a.lines = vec!["hi".to_string()];
    handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
    assert!(viewer_hint(false).contains("C"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test status_shows_visual_and_comment_count`
Expected: FAIL (hint has no `C`/visual tags yet)

- [ ] **Step 3: Implement paint (normalize anchor/cursor, line-expand, `selection_chunks` per wrap/`h_scroll` window, blue bg after highlight slice before cursor split) + sidebar split + `C` toggle + input row `Comment on <file:line>: <draft>` + `--VISUAL--` tags + `[n comments]`**

Covers Review Focus wrap/h_scroll clipping lines via `selection_chunks` tests in Task 1 plus manual render check.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test status_`
Expected: PASS

- [ ] **Step 5: Run full suite + clippy + manual smoke (`cargo run -- Cargo.toml`, `v3w`, `V`, `vi"`, `Enter`→type→`Enter`, `C`)**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: paint selection with sidebar and status"
```

Acceptance: selection visible in wrap/no-wrap, `C` shows `file: sL:sC → eL:eC snippet | note`, input row works.

---

## Execution order

Run Task 1 alone first. Tasks 2–4 each in their own run. Do not batch.
