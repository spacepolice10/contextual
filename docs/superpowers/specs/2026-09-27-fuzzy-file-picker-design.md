# Fuzzy File Picker (LazyVim `ff` style) — Design Spec
Date: 2026-09-27
Status: draft for review

## Goal
Replace the cwd-only `Mode::Picker` list with a Telescope-style fuzzy file finder: type a few chars, see the right file at the top instantly, preview it with syntax highlight, `Enter` to open. Must feel instant in repos up to ~50k files and stay dependency-light.

## Scope (agreed)
- Files-only v1. No live-grep, no buffers, no help/symbols. Architecture must not block a later `live-grep` mode.
- Recursive walk from cwd. Show hidden files (dotfiles), respect `.gitignore` / `.ignore` / `.git/info/exclude`. Works outside git repos too.
- Fuzzy engine: `nucleo` (Helix-grade matcher, pure Rust, background re-rank). No `fzf`/`fd` subprocess, no hand-rolled scorer.
- Telescope key feel: typing filters live, `Ctrl-j/k` + `Ctrl-p/n` + arrows move, `Enter` opens, `Esc` clears query first then quits/back, `q` quits.
- Preview pane reuses existing viewer pipeline (`highlight_file`, `build_display_lines`, `slice_spans`); never drops text on highlight failure.

## Approach: A — Native nucleo + ignore (chosen)
Rejected B (delegate to `fzf`/`fd` subprocess: flicker from tearing down `TerminalGuard` alt-screen + raw mode, hard external-binary dep, untestable ranking, can't paint matched chars with theme) and C (hand-rolled subsequence: zero deps but poor ranking, no separator/case bonuses, rewrite risk at 10k files).

## Architecture
- `src/picker.rs` — owns discovery + ranking (pure, testable):
  - `discover_files(root: &Path) -> Vec<FileEntry>`: `ignore::WalkBuilder` with `hidden(false)` (show dotfiles), `git_ignore(true)`, `parents(true)`, `require_git(false)`, `follow_links(false)`, skip dirs/symlink-dirs/errors, cap 50k entries. Keeps `FileEntry { name, path, size }` where `path` stays relative for display, `name` = file_name lossy.
  - `filter_files(entries: &[FileEntry], query: &str) -> Vec<ScoredMatch>`: `nucleo::Matcher` over relative-path string; returns `{ index, score, match_cols }` with `match_cols` as **char indices** into the display path for highlight painting (convert nucleo's byte indices via `char` mapping — never slice the path by bytes). Empty query → all entries in `list_files` sort order (name-sorted) with no highlight cols.
  - `clamp_selection` unchanged.
- `src/app.rs` — new `PickerState { query: String, filtered: Vec<ScoredMatch>, selected: usize, preview_scroll: usize }` held on `App` (or split `App::Picker` struct if it grows). `move_picker` retargets `filtered` length; typing resets `selected` to 0 only when the previous selection falls outside the new list (keeps stable otherwise); `preview_scroll` resets on selection change.
- `src/main.rs` — `handle()` Picker branch gains input keys (printable chars push, `Backspace` pops, `Ctrl-u` clears, `Ctrl-j/k`, `Ctrl-p/n`, `Up/Down`, `Enter`, `Esc`, `q`); Viewer `Esc`-back-to-picker re-runs `discover_files`. `render()` Picker branch becomes Telescope layout (below). No changes to `TerminalGuard` event loop timing.
- Deps: add `nucleo` + `ignore` only. No new tree-sitter features; `Lang::Markdown` stays plain fallback.

## Components / UI
Prompt row (top, Telescope `ff` order):
- Format: `> {query}▌  {filtered.len()}/{entries.len()}`; subtle `DarkGray` count. Cursor block at end (no mid-line editing in v1).
- Each keystroke re-ranks live; list shows top N fitting viewport; status keeps counts even at 0 (`> foo  0/1240  [no matches]`).

Results list (left ~50%):
- Row text: relative path (e.g. `src/highlight/langs.rs`), truncated to width by chars not bytes. Matched chars painted yellow bg / black fg (same style as `paint_search_ranges`); selected row gets `DarkGray` bg with matched-char style winning on those cells (cursor-wins ordering, same as viewer pipeline: slice spans → paint matches → paint selection).
- Show size suffix `(1.2k)` in `DarkGray` only when width allows; never shifts match columns (compute cols against path part only).

Preview pane (right ~50%, hidden when width < 80):
- Reads first ~200 lines of selected file lossy-UTF8 (no full `load_file` until `Enter`), highlights via `highlight_file(&text, detect(path), theme)` with `None` → plain `Span::raw` fallback; strips `\r`, pops trailing empty line from trailing `\n` per existing gotcha.
- Renders via `build_display_lines(lines, preview_w, wrap=true)` + `clamp_scroll(preview_scroll, ...)`; `preview_scroll` follows `j/k` in preview only when a `Ctrl-o`-style focus key is added — v1: preview is non-interactive, shows top of file.

Keys (Picker):
- Type / `Backspace` / `Ctrl-u` (clear) edit query. `Down`/`Ctrl-j`/`Ctrl-n` +1, `Up`/`Ctrl-k`/`Ctrl-p` −1 (wrap-around within filtered). `Enter` opens selected via existing `App::load_file(&path, true)`. `Esc`: non-empty query → clear query; empty query → quit (or back). `q` / `Ctrl-c` quit. `Ctrl-w` toggles wrap for preview (mirrors viewer).

## Data flow
1. Startup (no path arg) or Viewer `Esc`-back → `discover_files(".")` → `App::new_picker(files)` with `query=""`, `filtered=all`, `selected=0`.
2. Keystroke → update `query` → `filter_files` (nucleo re-rank on background thread; UI reads latest snapshot, never blocks) → clamp `selected` → reset `preview_scroll` if selection file changed → redraw.
3. Move keys → `selected = (selected ± 1) mod filtered.len()` (no-op when empty) → update preview → redraw.
4. `Enter` (filtered non-empty) → `App::load_file(&filtered[selected].path, true)`; empty list → no-op with `[no matches]` note.

## Matching details
- `nucleo` configured for path mode: filename bonus + separator bonus, smart-case (lowercase query = case-insensitive, any uppercase = sensitive) to mirror `search.rs` behavior.
- All highlight columns converted to char indices (`path.chars()` positions), never byte offsets; painting explodes spans to `(char, Style)` cells then re-merges, same as `paint_ranges`.
- Unicode paths: scoring on chars; truncation and `slice_spans`-style windowing by char count.

## Error handling
- Unreadable dirs / permission errors / symlink loops: skipped silently (same as current `list_files`); walker never aborts the picker.
- >50k files: keep first 50k in walk order, set `status_note="[truncated at 50k]"`; filtering still runs on the kept set.
- Binary / huge preview files: read lossy, cap preview at 200 lines / 100KB; `[binary preview]` note when NUL bytes dominate; `Enter` still opens full file via existing lossy path.
- Missing selection / empty results: `Enter` no-op, preview shows `No matches`.
- Narrow terminal (<80 cols): preview hidden, results take full width; never panic on `width==0`.

## Testing / quality
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
- Unit (`picker.rs`): discovery respects gitignore but shows dotfiles (temp dir with `.gitignore` + `.hidden` + `target/` fixture); hidden-but-ignored file excluded; empty query returns name-sorted; fuzzy order puts `src/main.rs` above `docs/domain-rs.md` for `mr`; match cols are valid char indices on unicode paths; `clamp_selection` bounds; truncation cap sets note.
- Unit (`app.rs`/`handle`): typing resets/clamps selection, `Esc` clears query first, `Enter` loads correct path with `from_picker=true`.
- Manual: 10k-file repo responsiveness (no input lag), resize with preview on/off, `Ctrl-j/k` wrap-around, unicode filenames, outside-git directory.

## Non-goals
Live-grep content search, buffers/recent/help pickers, multi-select, file ops (create/delete/rename), `fd`/`rg` delegation, preview scrolling/interaction, regex toggle, persistent history. Live-grep is the explicit v2 and must reuse `PickerState` + prompt layout.
