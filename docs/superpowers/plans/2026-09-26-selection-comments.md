# Selection, Yank History, and Line Comments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add vim-style visual selection, internal yank history, and overlay-only line comments with side panel to the terminal viewer.

**Architecture:** New pure logic modules (`selection.rs`, `comments.rs`) hold range math and comment store; `App` gains `visual`, `yanks`, `comments`, `focus` fields; `main.rs::handle`/`render` wires modes and paints selection + virtual comment rows + side panel. No file-on-disk mutation, no clipboard.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28, existing char-based indexing (`chars().count()`), `cargo test` inline `#[cfg(test)]` modules.

---

## File structure

- Create: `src/selection.rs` — `VisualKind::{Char,Line}`, `Visual{anchor_line,anchor_col,kind}`, `normalize()`, `word_bounds()`, `pair_bounds()`, `extract_text()`. Pure, testable, no ratatui.
- Create: `src/comments.rs` — `Comment{id,line,snapshot,body}`, `CommentStore{items,next_id}`, `add()`, `for_line()`, `compact_label()`. Pure, testable.
- Modify: `src/app.rs` — add `visual: Option<Visual>`, `yanks: Vec<Yank>`, `comments: CommentStore`, `focus: Focus::{Main,Side}`, `side_selected: usize`, `edit: Option<EditState>`. Yank = `struct Yank{text:Vec<String>,kind:VisualKind,source:String}`.
- Modify: `src/viewer.rs` — helper `is_in_selection(line,col,visual,cursor)` and `virtual_rows_for_line()` count for comment overlay rows.
- Modify: `src/main.rs` — `handle()` new keys (`v/V/Esc/y/c/Tab/B`), `render()` selection highlight pass, virtual comment rows, horizontal split for side panel, `status_line()` per-mode hotkeys.

Each task below is independently runnable and committable. Do not start next task in the same run.

---

### Task 1: Visual selection core

**Files:**
- Create: `src/selection.rs`
- Modify: `src/app.rs:12-31` (add `visual` field), `src/main.rs:96-152` (add `v/V/Esc` arms)
- Test: inline `src/selection.rs` tests

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn normalize_swaps_anchor_cursor() {
    let v = Visual { anchor_line: 3, anchor_col: 5, kind: VisualKind::Char };
    let (s, e) = normalize(&v, 1, 2);
    assert_eq!(s, (1, 2));
    assert_eq!(e, (3, 5));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test normalize_swaps_anchor_cursor`
Expected: FAIL with "cannot find" / compile error (module does not exist yet)

- [ ] **Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualKind { Char, Line }
#[derive(Debug, Clone, Copy)]
pub struct Visual { pub anchor_line: usize, pub anchor_col: usize, pub kind: VisualKind }
pub fn normalize(v: &Visual, cur_line: usize, cur_col: usize) -> ((usize, usize), (usize, usize)) {
    let a = (v.anchor_line, v.anchor_col);
    let b = (cur_line, cur_col);
    if a <= b { (a, b) } else { (b, a) }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test normalize_swaps_anchor_cursor`
Expected: PASS

- [ ] **Step 5: Add word + pair bounds + extract_text with tests** (same TDD cycle: `word_bounds("hello world", 7) == (6,11)`, `pair_bounds` same-line `(`…`)`, `extract_text` char vs line, unicode via `chars()`)

Run: `cargo test selection`
Expected: PASS, `cargo clippy -- -D warnings` clean

- [ ] **Step 6: Commit**

```bash
git add src/selection.rs src/app.rs src/main.rs
git commit -m "feat: add visual selection core with word and pair objects"
```

Acceptance: `v`/`V` highlights anchor→cursor range, `o` swaps ends, `Esc` exits. No yank yet.

---

### Task 2: Yank history buffer

**Files:**
- Modify: `src/app.rs` (add `yanks: Vec<Yank>`, cap 50)
- Modify: `src/main.rs` (`y` to yank, `B` to browse via existing picker pattern)
- Test: inline `src/app.rs` or `src/selection.rs` `extract_text` reuse

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn yank_pushes_history_and_caps() {
    let mut yanks: Vec<Yank> = vec![];
    push_yank(&mut yanks, Yank { text: vec!["a".into()], kind: VisualKind::Line, source: "f.rs".into() });
    assert_eq!(yanks.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test yank_pushes_history_and_caps`
Expected: FAIL (no `Yank`/`push_yank` yet)

- [ ] **Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone)]
pub struct Yank { pub text: Vec<String>, pub kind: VisualKind, pub source: String }
pub fn push_yank(yanks: &mut Vec<Yank>, y: Yank) {
    yanks.push(y);
    const CAP: usize = 50;
    if yanks.len() > CAP { yanks.drain(0..yanks.len()-CAP); }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test yank`
Expected: PASS

- [ ] **Step 5: Wire `y` key + `B` history picker + status note `"yanked 3 lines (slot 4)"`**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: add yank history buffer with picker browse"
```

Acceptance: visual + `y` stores snapshot, survives file switch, `B` lists history.

---

### Task 3: Comment model + inline editor (overlay only, multiple per line)

**Files:**
- Create: `src/comments.rs`
- Modify: `src/app.rs` (add `comments: CommentStore`, `edit: Option<EditState{line,buf,col}>`)
- Modify: `src/main.rs` (`c` opens editor under `cursor_line`, `Enter` saves, `Esc` cancels)
- Test: inline `src/comments.rs` tests

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn multiple_comments_per_line() {
    let mut s = CommentStore::default();
    s.add(5, "let x=1;".into(), "check type".into());
    s.add(5, "let x=1;".into(), "second note".into());
    assert_eq!(s.for_line(5).len(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test multiple_comments_per_line`
Expected: FAIL (module missing)

- [ ] **Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone)]
pub struct Comment { pub id: usize, pub line: usize, pub snapshot: String, pub body: String }
#[derive(Debug, Default)]
pub struct CommentStore { pub items: Vec<Comment>, next_id: usize }
impl CommentStore {
    pub fn add(&mut self, line: usize, snapshot: String, body: String) -> usize {
        let id = self.next_id; self.next_id += 1;
        self.items.push(Comment { id, line, snapshot, body });
        id
    }
    pub fn for_line(&self, line: usize) -> Vec<&Comment> {
        self.items.iter().filter(|c| c.line == line).collect()
    }
    pub fn compact_label(c: &Comment) -> String {
        format!("Ln {}: {:.40}", c.line + 1, c.body)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test comments`
Expected: PASS

- [ ] **Step 5: Wire inline editor row (in-place virtual row under logical line, `EditState{buf,col}`, ASCII input, `Enter`/`Esc`) + gutter `●` marker**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/comments.rs src/app.rs src/main.rs
git commit -m "feat: add overlay comments with inline editor"
```

Acceptance: `c` on a line opens editor below it, `Enter` keeps dimmed virtual row(s), multiple per line stack, file on disk untouched.

---

### Task 4: Side panel + focus switching

**Files:**
- Modify: `src/main.rs::render` (horizontal split `70% main | 30% side` when `!comments.items.is_empty()` or `focus==Side`), `src/viewer.rs` (virtual-row offset helper)
- Modify: `src/app.rs` (add `focus`, `side_selected`)
- Test: `viewer.rs` offset test

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn virtual_row_offset_counts_comments() {
    assert_eq!(display_offset_for_line(5, &[(5, 2), (3, 1)]), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test virtual_row_offset`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation** (count comments on lines `< target`)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test virtual_row_offset`
Expected: PASS

- [ ] **Step 5: Wire `Tab` focus toggle, `j/k` in Side, `Enter` jumps cursor + `ensure_cursor_visible`, `d` deletes selected comment**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/viewer.rs src/app.rs
git commit -m "feat: add comment side panel with focus switching"
```

Acceptance: side list shows `compact_label`, `Tab` switches, `Enter` jumps to line.

---

### Task 5: Status bar hotkeys per mode

**Files:**
- Modify: `src/main.rs::status_line` (or inline bar in `render`)
- Test: `main.rs` `status_tests`-style unit test

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn status_shows_visual_keys() {
    assert!(status_for(&mode_visual(), &focus_main()).contains("y:yank"));
    assert!(status_for(&mode_viewer_side(), &focus_side()).contains("Tab:main"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test status_shows_visual_keys`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
fn status_for(app: &App) -> String {
    match (&app.visual, &app.edit, &app.focus) {
        (Some(v), _, _) if v.kind == VisualKind::Line => " VISUAL-LINE y:yank c:comment Esc:cancel ".into(),
        (Some(_), _, _) => " VISUAL v:off y:yank c:comment o:swap Esc:cancel ".into(),
        (None, Some(_), _) => " COMMENT Enter:save Esc:cancel ".into(),
        (None, None, Focus::Side) => " COMMENTS j/k:move Enter:jump d:del Tab:main ".into(),
        _ => " j/k:move v:visual V:line B:yanks c:comment Tab:side q:quit ".into(),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test status_`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add per-mode status bar hotkeys"
```

Acceptance: bar always reflects current mode/focus, no stale hints.

---

## Execution order

Run Task 1 alone first. Tasks 2–5 each in their own run. Do not batch.
