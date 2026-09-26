# Vim Visual Select + Inline Comments with Sidebar — Design Spec
Date: 2026-09-27
Status: approved

## Goal
Add vim-style text selection to `contextual` viewer for annotation: select with `v`/`V`, counts, and `i`/`a` delimiter objects; press `Enter` to write a short note in a small input; save `{file, span, snippet, note}` to a memory-only session buffer; browse via a toggleable sidebar.

## Scope (agreed)
- Visual char-wise `v`, line-wise `V`; counts on navigation (`5j`, `v3w`); text-objects `i`/`a` + `{}()[]<>"'```.
- `Enter` on active selection opens single-line input below; `Enter` saves, `Esc` cancels.
- Memory-only `Vec<Comment>`; lost on quit. Sidebar toggle `C`, no focus/scroll in v1.
- No clipboard/yank, no persistence, no block-wise `Ctrl-V`, no delete operators. YAGNI (deferred).

## Approach: 1 — Visual+sidebar MVP (chosen)
Rejected 2 (full operator-pending with `d/y/c`, focusable sidebar — doubles `handle()` for little v1 gain) and 3 (selection-only spike with no sidebar — skips the requested list UI).

## Architecture
- `src/app.rs` — extend `App`: `visual: Option<Selection>`, `pending_count: Option<usize>`, `commenting: Option<PendingComment>`, `comments: Vec<Comment>`, `show_sidebar: bool`.
  - `SelectKind::{Char, Line}`, `Selection{anchor: (usize, usize), kind: SelectKind}` (logical line, char-col).
  - `Comment{id: usize, file: String, start: (usize, usize), end: (usize, usize), snippet: String, note: String}`.
  - `PendingComment{snippet: String, start: (usize, usize), end: (usize, usize), draft: String}`.
- `src/select.rs` (new, pure) — `normalize(anchor, cursor, kind) -> (start, end)`; `extract_text(lines, start, end) -> String`; `find_delim_pair(lines, cursor, open, close, inner: bool) -> Option<(start, end)>`; `selection_chunks(...)` helper mapping a normalized range to visible-chunk ranges (mirrors `search::chunk_match_ranges`). All char-based, never byte-slice; `\r` stripped like `highlight_file`.
- `src/viewer.rs` — unchanged (reuse `display_row_for_cursor`, `clamp_scroll`, `content_size`).
- `src/main.rs` — `handle()` gains count accumulator + visual branch + `i`/`a` object pending state + commenting input branch + `C` toggle; `render()` gains selection paint + sidebar + input line.
- No new dependencies.

## Components / UI
Selection paint (reuses `paint_search_ranges` pattern, blue bg instead of yellow):
- Normalize anchor<->cursor (either direction); line-wise expands to full lines first.
- Convert to per-chunk ranges for current wrap/`h_scroll` window; paint after highlight slice, before cursor split so cursor cell still wins.
- Works in wrap and nowrap since range is logical `(line, col)`.

Comment input (status line 1 while `commenting`):
- Format: `Comment on <file:line>: <draft>▌`; line 2 hint `Enter save · Esc cancel`.
- Typing appends to `draft`, `Backspace` pops; `Enter` with any draft (empty allowed) saves; `Esc` returns to visual.

Sidebar (body splits `[text | 35-col sidebar]` only when `show_sidebar`):
- Title `comments (C)`; rows `file: sL:sC → eL:eC`, truncated snippet (first 40 chars + `…`), `| note`.
- Empty: `No comments — v select, Enter comment`. Shows last N fitting height; no scroll/focus in v1.

Keys (Viewer):
- `1-9` accumulate `pending_count`; `0` = col-0 motion when no pending, else digit. `Esc` clears pending + visual + commenting (commenting first).
- `v`/`V` enter/exit visual (anchor = cursor). Motions `hjkl wWbBeE 0^$ gG` + half-page move cursor by count, anchor stays.
- `i`/`a` in visual + delimiter replaces selection via `find_delim_pair`; miss sets `status_note="no match"`, keeps selection.
- `Enter` with visual starts commenting; `Enter` with no visual = noop. `C` toggles sidebar.

Status/hint:
- Mode tags `--VISUAL--` / `--VISUAL LINE--` / `Comment…`; hint adds `v/V select · Enter comment · C sidebar · [n comments]`.

## Data flow
1. `v`/`V` → `visual=Some{anchor: cursor, kind}`.
2. Motion (+count) or `i`/`a`+delim → update cursor (and selection on next paint).
3. `Enter` → `commenting=Some{snippet: extract_text(...), span, draft: ""}`.
4. Keystroke → `draft` update → redraw input line.
5. `Enter` → push `Comment{file: filename, span, snippet, note: draft}`, clear visual+commenting. `Esc` → drop `commenting`, keep visual.
6. `C` → `show_sidebar=!show_sidebar`; sidebar reads `comments` each frame.

## Error handling
- Empty file: `v`/`V`/`Enter` noop. Delim miss: transient `status_note`, no state loss.
- Unicode: char indices only; slicing via `chars()`.
- Counts clamp to file bounds via existing `set_cursor` clamping.

## Testing / quality
- `select.rs` unit tests: normalize both directions, line-expand, extract multiline, delim same-line/nested/miss/cross-line/quotes.
- `handle()` tests: `v3w`, `5j` in visual, `vi"` selects inner quotes, `Enter` saves comment with file ref, `Esc` cancel paths, `C` toggle.
- `cargo test`, `cargo clippy -- -D warnings` clean.
