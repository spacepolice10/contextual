# Fuzzy File Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace cwd-only picker with recursive Telescope-style fuzzy finder with preview.

**Architecture:** `picker.rs` owns discovery (`ignore` walker) + ranking (`nucleo` background worker); `app.rs` owns `PickerState`; `main.rs` owns Picker key handling + Telescope render reusing highlight/viewer helpers.

**Tech Stack:** Rust, `ratatui` 0.29, `crossterm` 0.28, `nucleo` 0.5, `ignore` 0.4, `nucleo-matcher` (via `nucleo` re-export for `CaseMatching`/`Normalization`)

**Spec:** `docs/superpowers/specs/2026-09-27-fuzzy-file-picker-design.md`

## Global Constraints
- All text indexing is char-based (`chars().count()`); never byte-slice except guarded `text.get(start..end)` inside tree-sitter range.
- `highlight_file` must never drop text: backend failure or `None` config → `None` plain `Span::raw` fallback; trailing `\n` pops extra empty line; `\r` stripped per chunk.
- `Lang::Markdown` stays plain fallback (no grammar).
- Render order: slice styled spans → paint search/match ranges (yellow bg) → paint selection → cursor cell wins.
- Keep `cargo clippy -- -D warnings` clean; tests are inline `#[cfg(test)]` modules, no `tests/` dir.
- Preview reads are capped (200 lines / 100KB); discovery caps at 50k entries with `[truncated at 50k]` note.

## Review Focus
- Unicode path with emoji + CJK: query `🌍` highlights the right char cell, no panic, columns stay char-based.
- Symlink loop + unreadable dir in tree: picker still opens, entries skip silently, no hang.
- 50k-file repo + fast typing: UI never blocks; latest keystroke wins, count shows `n/50000`.
- Narrow terminal (width < 80): preview hidden, results full-width, no panic on width 0.
- Binary file selected (NUL bytes): preview shows `[binary preview]` note, `Enter` still opens lossy full file.

---

### Task 1: Recursive discovery with gitignore + hidden shown

**Files:**
- Modify: `Cargo.toml` (add `ignore = "0.4"`)
- Modify: `src/picker.rs`
- Test: inline `src/picker.rs::tests`

**Interfaces:**
- Consumes: existing `FileEntry { name: String, path: PathBuf, size: u64 }`, `list_files`, `clamp_selection`.
- Produces: `pub fn discover_files(root: &Path) -> (Vec<FileEntry>, bool)` returning entries + `truncated` flag; `pub const MAX_PICKER_FILES: usize = 50_000`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn discover_respects_gitignore_but_shows_hidden() { /* temp dir: .gitignore ignoring ignored.txt, files: kept.txt, .hidden, ignored.txt → names == [".hidden", "kept.txt"] */ }
#[test]
fn discover_recursive_and_caps_truncation_flag() { /* nested subdir file found; large synthetic count → truncated == true implies len == MAX_PICKER_FILES */ }
#[test]
fn discover_skips_dirs_and_errors() { /* subdir itself not listed */ }
#[test]
fn discover_skips_symlink_loop() { /* symlink pointing at parent dir does not hang and is not listed as file */ }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test picker -- --nocapture`
Expected: FAIL — `discover_files` / `MAX_PICKER_FILES` not defined.

- [ ] **Step 3: Implement `discover_files(root: &Path) -> (Vec<FileEntry>, bool)` in `src/picker.rs`**

Use `ignore::WalkBuilder::new(root).hidden(false).git_ignore(true).parents(true).require_git(false).follow_links(false).build()`; keep regular files only, `name` from `file_name` lossy, `size` via metadata (0 on error), sort by `name`, stop at `MAX_PICKER_FILES` and set `truncated=true`. Add `ignore = "0.4"` to `[dependencies]`.

- [ ] **Step 4: Run tests**

Run: `cargo test picker`
Expected: PASS. Also run `cargo clippy -- -D warnings`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/picker.rs
git commit -m "feat(picker): recursive discovery honoring gitignore, hidden shown"
```

### Task 2: Nucleo fuzzy ranking with char-index match columns

**Files:**
- Modify: `Cargo.toml` (add `nucleo = "0.5"`)
- Modify: `src/picker.rs`
- Test: inline `src/picker.rs::tests`

**Interfaces:**
- Consumes: `FileEntry` list + `MAX_PICKER_FILES` from Task 1.
- Produces: `pub struct ScoredMatch { pub entry_idx: usize, pub score: u32, pub cols: Vec<usize> }` (cols = char indices into display relative-path); `pub fn filter_files(entries: &[FileEntry], query: &str) -> Vec<ScoredMatch>`; `pub fn display_path(entry: &FileEntry) -> String`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn empty_query_returns_name_sorted_without_cols() { /* filter_files(entries, "") → entry_idx order matches name sort, all cols empty */ }
#[test]
fn fuzzy_prefers_filename_and_reports_char_cols() { /* entries src/main.rs + docs/domain-rs.md, query "mr" → first is src/main.rs, cols non-empty and valid char indices */ }
#[test]
fn unicode_cols_are_char_not_byte() { /* path "héllo_🌍.txt", query "🌍" → cols == [6], painting slices that char */ }
#[test]
fn smart_case_upper_is_sensitive() { /* query "Main" does not match "main.rs" when case differs */ }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test filter_files`
Expected: FAIL — `ScoredMatch` / `filter_files` not defined.

- [ ] **Step 3: Implement ranking in `src/picker.rs`**

One line: use `nucleo::Nucleo` worker (inject display paths once per recompute, `pattern::Pattern::parse(query, CaseMatching::Smart, Normalization::Smart)`, `snapshot.matched_items()` for score order) — or `nucleo_matcher::{Matcher, Config::DEFAULT.match_paths(), Pattern::new(..., AtomKind::Fuzzy)}` sync path if profiling shows <5ms at 50k; convert nucleo byte indices to char indices via `path.char_indices()` map (never byte-slice). Add `nucleo = "0.5"` dep. `display_path` = relative path with `/` separators, lossy.

- [ ] **Step 4: Run tests**

Run: `cargo test picker`
Expected: PASS (unicode + smart-case + ordering). `cargo clippy -- -D warnings`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/picker.rs
git commit -m "feat(picker): nucleo fuzzy ranking with char-index highlights"
```

### Task 3: PickerState in App

**Files:**
- Modify: `src/app.rs`
- Test: inline `src/app.rs::tests`

**Interfaces:**
- Consumes: `FileEntry`, `ScoredMatch`, `filter_files`, `discover_files` from Tasks 1–2.
- Produces: `pub struct PickerState { pub query: String, pub filtered: Vec<ScoredMatch>, pub selected: usize, pub preview_scroll: usize, pub truncated: bool }`; `App { pub picker: PickerState }` (keep `files`, `picker_index` as compat shims or migrate); `impl App { pub fn picker_recompute(&mut self); pub fn picker_move(&mut self, delta: isize); pub fn picker_clear(&mut self); }`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn typing_recomputes_and_clamps_selection() { /* set query "mr" → filtered non-empty, selected == 0 */ }
#[test]
fn move_wraps_around() { /* picker_move(1) past end wraps to 0; move(-1) from 0 wraps to len-1; empty list no-op */ }
#[test]
fn clear_resets_query_and_selection() { /* picker_clear → query "", selected 0 */ }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test picker_`
Expected: FAIL — `PickerState` / methods missing.

- [ ] **Step 3: Implement `PickerState` + helpers in `src/app.rs`**

`new_picker(files)` builds `picker { query: "", filtered: all-indices, selected: 0, preview_scroll: 0, truncated }`; `picker_recompute` calls `filter_files(&self.files, &self.picker.query)`, clamps `selected` (reset to 0 only if out of bounds), resets `preview_scroll` on file change; `picker_move` wraps with `rem_euclid`, no-op when empty.

- [ ] **Step 4: Run tests**

Run: `cargo test app::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(picker): PickerState with recompute and wrap-around selection"
```

### Task 4: Picker key handling (Telescope feel)

**Files:**
- Modify: `src/main.rs` (`handle()` Picker branch)
- Test: inline `src/main.rs::handle_tests`

**Interfaces:**
- Consumes: `App::picker_recompute`, `picker_move`, `picker_clear` from Task 3.
- Produces: Picker key map (no new public API; behavior contract below).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn picker_typing_filters_and_ctrl_u_clears() { /* Char('m'), Char('r') → query "mr"; Ctrl-u → query "" */ }
#[test]
fn picker_ctrl_jk_move_and_wrap() { /* Ctrl-j / Ctrl-k move selected ±1 with wrap */ }
#[test]
fn picker_esc_clears_first_then_quits() { /* Esc with query → clears, returns false; Esc empty → returns true */ }
#[test]
fn picker_enter_loads_selected() { /* Enter with non-empty filtered → mode Viewer, filename == selected path, from_picker true; empty filtered → stays Picker */ }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test picker_`
Expected: FAIL on new key behaviors (typing currently ignored, Esc quits immediately).

- [ ] **Step 3: Implement Picker branch in `handle()`**

Printable `Char(c)` (no Ctrl/Alt) pushes + recomputes; `Backspace` pops + recomputes; `Ctrl-u` clears; `Down`/`Ctrl-j`/`Ctrl-n` → `picker_move(1)`; `Up`/`Ctrl-k`/`Ctrl-p` → `picker_move(-1)`; `Enter` loads `files[filtered[selected].entry_idx]` via `App::load_file(path, true)` (empty → no-op); `Esc` non-empty → clear, empty → `Ok(true)`; `q`/`Ctrl-c` → `Ok(true)`.

- [ ] **Step 4: Run tests**

Run: `cargo test handle_tests`
Expected: PASS. `cargo clippy -- -D warnings`.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat(picker): telescope keymap with live filter and esc-clear"
```

### Task 5: Telescope render — prompt + results + preview

**Files:**
- Modify: `src/main.rs` (`render()` Picker branch)
- Test: inline pure helpers if extracted (e.g. `picker_prompt_line`, `truncate_path_to_width`); snapshot via existing `status_line` pattern.

**Interfaces:**
- Consumes: `PickerState`, `display_path`, `ScoredMatch.cols`, `highlight::{detect, highlight_file, slice_spans}`, `viewer::{build_display_lines, clamp_scroll, content_size}`.
- Produces: Telescope layout (prompt top, results left, preview right ≥80 cols); matched chars yellow bg/black fg with selection bg underneath (selection loses on matched cells, cursor-wins ordering preserved).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn prompt_shows_count() { /* picker_prompt_line("mr", 3, 120) contains "> mr" and "3/120" */ }
#[test]
fn preview_cap_truncates() { /* read_preview_lines(path, 200) never exceeds 200 lines even for 1000-line file */ }
#[test]
fn preview_binary_shows_note() { /* file with NUL bytes → ([lines], "[binary preview]") note, still returns capped lines */ }
#[test]
fn narrow_width_hides_preview() { /* layout_preview_visible(79) == false, (80) == true */ }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test picker_prompt`
Expected: FAIL — helpers not defined.

- [ ] **Step 3: Implement Telescope render**

Top prompt `> query▌  n/m` (+ `[no matches]` / `[truncated at 50k]`); results `List` over visible `filtered` window with per-row matched-char spans (explode display path to `(char, Style)` cells, paint `cols` yellow, then selection-row `DarkGray` bg on unpainted cells); preview pane (right 50% when width ≥ 80) via capped lossy read (200 lines/100KB, `[binary preview]` on NUL) + `highlight_file` → `None` falls back to `Span::raw` + `build_display_lines(wrap=true)`; empty filtered → `No matches` preview.

- [ ] **Step 4: Run tests + manual render check**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: PASS. Manual: `cargo run --` in repo, type `mr`, check top hit + preview.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat(picker): telescope layout with match highlight and preview"
```

### Task 6: Wiring + polish (startup, Esc-back, caps, counts)

**Files:**
- Modify: `src/main.rs` (`main()` startup + Viewer Esc-back branch), `src/app.rs` (compat: keep or remove `picker_index` after migration)
- Test: inline `handle_tests` + `app::tests`

**Interfaces:**
- Consumes: everything from Tasks 1–5.
- Produces: `cargo run` (no args) discovers + opens picker; Viewer `Esc` (from_picker) re-discovers and returns to picker; status counts always `filtered/total`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn esc_from_viewer_returns_to_picker() { /* Viewer from_picker + Esc → Mode::Picker with files non-empty */ }
#[test]
fn truncated_note_set() { /* picker with truncated=true → status contains "50k" */ }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test esc_from_viewer`
Expected: FAIL if Esc-back still uses old `list_files` or drops truncation flag.

- [ ] **Step 3: Implement wiring**

Startup + Esc-back call `discover_files(".")` (propagate `truncated` into `PickerState`); remove or shim `picker_index` so `move_picker` delegates to `picker_move`; ensure `viewport_h` split accounts for prompt row; `width==0` guards (results-only, no preview, no panic).

- [ ] **Step 4: Full verification**

Run: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
Expected: all PASS. Manual: `cargo run --` → type, `Ctrl-j/k`, `Enter`, `Esc`-back, narrow resize.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/app.rs
git commit -m "feat(picker): wire discovery, esc-back, and truncation notes"
```
