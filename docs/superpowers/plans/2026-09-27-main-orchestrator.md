# Main-as-orchestrator Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `src/main.rs` into feature modules so `main` is a thin orchestrator.

**Architecture:** Verbatim code moves first (no logic edits), then one small `App` method per step behind the moved call sites; every step ends green.

**Tech Stack:** Rust binary crate, ratatui 0.29, crossterm 0.28, clap 4.5 derive, `cargo test`, `cargo clippy -- -D warnings`.

**Spec:** `docs/superpowers/specs/2026-09-27-main-orchestrator-design.md`

## Global Constraints

- No behavior change; render order slice → search → selection → cursor wins is fixed.
- Input priority commenting → pending-object → searching → normal; Esc priority pending/visual → committed search → quit/back.
- All text indexing char-based, never byte-slice except guarded tree-sitter `Source` range.
- `highlight_file` never drops text; `None` → `Span::raw` fallback.
- Preview read byte-capped (`PREVIEW_MAX_BYTES = 100 * 1024`), infallible.
- Every step ends with `cargo test` green and `cargo clippy -- -D warnings` clean.

## Review Focus

- Empty file opened directly (`lines == []`): viewer renders gutter + status, `v`/`Enter` stay noop, no panic.
- Terminal narrower than gutter+border (`content_size` saturates): text width floors at 1, height may be 0, no underflow.
- Search query with multibyte chars (e.g. `🌍`): match cols are char indices, paint clips to visible chunk.
- Unreadable preview path: `read_preview_lines` returns empty lines, render still shows title row.
- Highlight line-count mismatch vs text lines: renderer falls back to plain `Span::raw`, never drops a row.

---

### Task 1: `cli.rs` + `tui.rs` bootstrap extract

**Files:**
- Create: `src/cli.rs`, `src/tui.rs`
- Modify: `src/main.rs` (use new modules, delete moved items)
- Test: existing build + `cargo test cli_smoke` (no new tests; compile-gated)

**Interfaces:**
- Consumes: nothing.
- Produces: `pub struct Cli { pub path: Option<PathBuf> }` + `Cli::parse()` (clap derive, unchanged); `pub struct TerminalGuard` + `TerminalGuard::enter() -> anyhow::Result<Self>` (Drop restores terminal).

- [ ] **Step 1: Create `src/cli.rs` with `Cli` moved verbatim from `src/main.rs:21-26`.**
- [ ] **Step 2: Create `src/tui.rs` with `TerminalGuard` moved verbatim from `src/main.rs:28-43`.**
- [ ] **Step 3: Rewire `src/main.rs` to `mod cli; mod tui;` and `cli::Cli::parse()` / `tui::TerminalGuard::enter()`.**
- [ ] **Step 4: Run `cargo test` — Expected: all 62 pass.**
- [ ] **Step 5: Run `cargo clippy -- -D warnings` — Expected: clean.**
- [ ] **Step 6: Commit.**

```bash
git add src/cli.rs src/tui.rs src/main.rs
git commit -m "refactor: extract cli and tui bootstrap from main"
```

### Task 2: Pure `ui` leaves — paint, status, sidebar

**Files:**
- Create: `src/ui/mod.rs`, `src/ui/paint.rs`, `src/ui/status.rs`, `src/ui/sidebar.rs`
- Modify: `src/main.rs` (delete moved fns, `use crate::ui::...`)
- Test: moved `status_tests` paint/sidebar/status cases

**Interfaces:**
- Consumes: `ratatui::text::{Line, Span}`, `ratatui::style::{Color, Style}`, `crate::app::App`, `crate::select::Selection`.
- Produces: `pub fn paint_search_ranges(spans: Vec<Span<'static>>, ranges: &[(usize, usize)]) -> Vec<Span<'static>>`; `pub fn paint_selection_ranges(...)` (same sig); `pub(crate) fn paint_ranges(spans, ranges, style)`; `pub fn viewer_hint(searching: bool) -> &'static str`; `pub fn visual_tag(visual: Option<Selection>) -> &'static str`; `pub fn comments_tag(n: usize) -> String`; `pub fn comment_prompt(file: &str, line: usize, draft: &str) -> String`; `pub fn status_line(width: usize, left: &str, right: &str) -> Line<'static>`; `pub fn sidebar_lines(app: &App, height: usize, width: usize) -> Vec<Line<'static>>`.

- [ ] **Step 1: Move `paint_ranges` + 2 wrappers verbatim into `src/ui/paint.rs`; move `status_tests::selection_paint_uses_blue_bg` with it.**
- [ ] **Step 2: Run `cargo test selection_paint` — Expected: PASS.**
- [ ] **Step 3: Move `viewer_hint`, `visual_tag`, `comments_tag`, `comment_prompt`, `status_line` verbatim into `src/ui/status.rs` + their `status_tests` cases.**
- [ ] **Step 4: Run `cargo test status_` — Expected: PASS.**
- [ ] **Step 5: Move `sidebar_lines` verbatim into `src/ui/sidebar.rs` + `sidebar_*` tests.**
- [ ] **Step 6: Run `cargo test sidebar` — Expected: PASS (pins Review Focus empty-comments help row).**
- [ ] **Step 7: Run full `cargo test` + `cargo clippy -- -D warnings` — Expected: green/clean.**
- [ ] **Step 8: Commit.**

```bash
git add src/ui src/main.rs
git commit -m "refactor: extract ui paint, status and sidebar leaves"
```

### Task 3: File-domain preview helpers into `picker.rs`

**Files:**
- Modify: `src/picker.rs` (append moved items), `src/main.rs` (delete, re-export use)
- Test: moved `picker_preview_*` + `picker_prompt_*` + `truncated_flag` cases

**Interfaces:**
- Consumes: `std::path::Path`.
- Produces: `pub const PREVIEW_MAX_BYTES: usize` (= 100*1024); `pub fn read_preview_lines(path: &Path, max_lines: usize) -> (Vec<String>, Option<String>)`; `pub fn picker_prompt_line(query: &str, filtered: usize, total: usize, truncated: bool) -> String`; `pub fn picker_preview_visible(width: usize) -> bool`.

- [ ] **Step 1: Move `PREVIEW_MAX_BYTES`, `read_preview_lines`, `picker_prompt_line`, `picker_preview_visible` verbatim into `src/picker.rs` with `pub` visibility; move their 6 `status_tests` cases into `picker.rs::tests`.**
- [ ] **Step 2: Run `cargo test picker_preview` — Expected: PASS (pins Review Focus unreadable-path + 100KB-cap + binary-note).**
- [ ] **Step 3: Run `cargo test picker_prompt` + `cargo test truncated_flag` — Expected: PASS.**
- [ ] **Step 4: Run full `cargo test` + clippy — Expected: green/clean.**
- [ ] **Step 5: Commit.**

```bash
git add src/picker.rs src/main.rs
git commit -m "refactor: move preview helpers into picker domain"
```

### Task 4: `ui/picker.rs` — picker pane render

**Files:**
- Create: `src/ui/picker.rs`
- Modify: `src/main.rs` (picker arm of `render` becomes one call), `src/ui/mod.rs` (dispatcher)
- Test: existing picker `handle_tests` (unchanged behavior); no new tests

**Interfaces:**
- Consumes: `crate::app::App`, `crate::picker::{display_path}`, `crate::highlight::{detect, highlight_file, slice_spans}`, `crate::viewer::{build_display_lines, clamp_scroll}`, `crate::ui::paint::{paint_ranges, paint_search_ranges}`.
- Produces: `pub fn render_picker(f: &mut ratatui::Frame, app: &mut App, area: ratatui::layout::Rect)`.

- [ ] **Step 1: Move picker arm of `render()` (`src/main.rs:839-993`) verbatim into `render_picker`; keep `app.picker.preview_scroll` mutation inside.**
- [ ] **Step 2: Run `cargo test picker_` — Expected: PASS.**
- [ ] **Step 3: Manual render check via existing `handle_tests::picker_enter_loads_selected` path — Expected: PASS in full suite.**
- [ ] **Step 4: Run full `cargo test` + clippy — Expected: green/clean.**
- [ ] **Step 5: Commit.**

```bash
git add src/ui/picker.rs src/ui/mod.rs src/main.rs
git commit -m "refactor: extract picker pane render"
```

### Task 5: `ui/viewer.rs` — viewer pane render

**Files:**
- Create: `src/ui/viewer.rs`
- Modify: `src/main.rs`, `src/ui/mod.rs`
- Test: moved `render_smoke_visual_sidebar_commenting`

**Interfaces:**
- Consumes: same as Task 4 plus `crate::viewer::{build_display_lines, clamp_scroll, clamp_hscroll, content_size, display_row_for_cursor}`, `crate::search::{chunk_match_ranges, scroll_for_match}`, `crate::select::{normalize, selection_chunks, SelectKind}`, `crate::ui::{paint, status, sidebar}`.
- Produces: `pub fn render_viewer(f: &mut ratatui::Frame, app: &mut App, area: ratatui::layout::Rect)`.

- [ ] **Step 1: Move viewer arm of `render()` (`src/main.rs:995-1301`) verbatim into `render_viewer`, including scroll/cursor sync, `line_matches` map, wrap/hscroll windows, gutter, sidebar slot, 2-row status bar.**
- [ ] **Step 2: Move `render_smoke_visual_sidebar_commenting` into `ui/viewer.rs::tests` verbatim.**
- [ ] **Step 3: Run `cargo test render_smoke` — Expected: PASS (pins Review Focus highlight-mismatch fallback, blue-cell paint, narrow-terminal path via TestBackend 80x24).**
- [ ] **Step 4: Run full `cargo test` + clippy — Expected: green/clean.**
- [ ] **Step 5: Commit.**

```bash
git add src/ui/viewer.rs src/ui/mod.rs src/main.rs
git commit -m "refactor: extract viewer pane render"
```

### Task 6: `input/*` — keyboard dispatch split

**Files:**
- Create: `src/input/mod.rs`, `src/input/picker.rs`, `src/input/viewer.rs`
- Modify: `src/main.rs` (dispatcher call only), `src/tui.rs` if loop references `handle`
- Test: moved `handle_tests` split by mode

**Interfaces:**
- Consumes: `crate::app::{App, Mode}`, `crate::picker`, `crate::search`, `crate::select`, `crate::viewer`.
- Produces: `pub fn handle(app: &mut App, code: KeyCode, mods: KeyModifiers) -> anyhow::Result<bool>` in `input/mod.rs`; `pub(super) fn handle_picker(...)` / `pub(super) fn handle_viewer(...)` with identical signatures.

- [ ] **Step 1: Move `MAX_COUNT` + `handle()` dispatcher shell into `src/input/mod.rs`; picker branch verbatim into `src/input/picker.rs` + picker `handle_tests`.**
- [ ] **Step 2: Run `cargo test picker_` — Expected: PASS.**
- [ ] **Step 3: Move viewer commenting/pending-object/searching/normal arms verbatim into `src/input/viewer.rs` + remaining `handle_tests` (pins Review Focus empty-file noop `enter_noop_without_visual_and_on_empty_file`, huge-count cap `huge_count_is_capped`, multibyte search nav, `esc_*` priority).**
- [ ] **Step 4: Run `cargo test` full — Expected: 62 pass.**
- [ ] **Step 5: Run clippy — Expected: clean.**
- [ ] **Step 6: Commit.**

```bash
git add src/input src/main.rs src/tui.rs
git commit -m "refactor: split input dispatch by picker and viewer"
```

### Task 7: `App` API — `open_selected_entry`, comment, search step

**Files:**
- Modify: `src/app.rs`, `src/input/picker.rs`, `src/input/viewer.rs`
- Test: existing `handle_tests` unchanged (they call `handle`, not the new methods directly)

**Interfaces:**
- Consumes: existing `App` fields.
- Produces: `impl App { pub fn open_selected_entry(&mut self) -> anyhow::Result<bool>; pub fn commit_comment_draft(&mut self); pub fn cancel_commenting(&mut self); pub fn step_search(&mut self, dir: isize); }` — bodies are the exact code currently inline in `handle()` (Enter-picker load, comment save block, `n`/`N` index advance + cursor set).

- [ ] **Step 1: Add `App::open_selected_entry` (moved Enter-picker block); call it from `input/picker.rs`.**
- [ ] **Step 2: Run `cargo test picker_enter` — Expected: PASS (pins Review Focus selection-follows-highlight, empty-list noop).**
- [ ] **Step 3: Add `commit_comment_draft` / `cancel_commenting` (moved Enter/Esc-comment blocks); call from `input/viewer.rs`.**
- [ ] **Step 4: Run `cargo test comment or vi_quote or empty_note or esc_drops` — Expected: PASS.**
- [ ] **Step 5: Add `step_search(dir: isize)` (moved `n`/`N` advance); call from `input/viewer.rs`.**
- [ ] **Step 6: Run full `cargo test` + clippy — Expected: green/clean.**
- [ ] **Step 7: Commit.**

```bash
git add src/app.rs src/input
git commit -m "refactor: hide App fields behind open/comment/search methods"
```

### Task 8: Thin `main.rs` + final verification

**Files:**
- Modify: `src/main.rs` (wiring only)
- Test: whole suite + manual smoke

**Interfaces:**
- Consumes: `crate::{cli, tui, input, ui, app, picker}`.
- Produces: `fn main() -> anyhow::Result<()>` unchanged behavior; `main.rs` target ~70 lines (mods + `main` + nothing else).

- [ ] **Step 1: Delete all remaining helpers from `main.rs`; leave `mod` decls + `main()` loop calling `input::handle` and `ui::render`.**
- [ ] **Step 2: Run `cargo test` — Expected: 62 pass.**
- [ ] **Step 3: Run `cargo clippy -- -D warnings` — Expected: clean.**
- [ ] **Step 4: Run `cargo build --no-default-features --features lang-rust` — Expected: success (per AGENTS.md minimal-feature check).**
- [ ] **Step 5: Commit.**

```bash
git add src/main.rs
git commit -m "refactor: main is now a thin orchestrator"
```
