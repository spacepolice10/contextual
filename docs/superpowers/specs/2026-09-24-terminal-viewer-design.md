# Terminal Text Viewer — Design Spec
Date: 2026-09-24
Status: approved

## Goal
Extremely minimal but robust terminal app in Rust that opens text files,
renders them effectively with scrolling. Clean, idiomatic, best-practices code.

## Scope (agreed)
- Open + scroll (vertical). Single file view.
- Base chrome: line numbers, filename in status bar, scroll position.
- Clear error if file cannot be opened (missing, permission, directory, invalid UTF-8).
- Wrap toggle via hotkey (`w`).
- File input: `contextual <path>` opens directly; no arg shows simple picker
  listing openable files in launch directory.
- No search, no editing, no syntax highlight, no tabs. YAGNI.

## Approach: A — Small modular (chosen)
Rejected B (single-file, untestable) and C (streaming for 100MB+ files, overkill).

## Architecture
Binary crate `contextual`, edition 2021. <500 LOC.
- `src/main.rs` — CLI parse (clap), terminal init/restore (RAII guard),
  event loop, top-level error print.
- `src/app.rs` — `enum Mode { Picker, Viewer }`, shared `App` state, key dispatch.
- `src/picker.rs` — pure logic: scan cwd for regular files, sort by name,
  selection index clamp. IO isolated for testability.
- `src/viewer.rs` — pure logic: `wrap_lines()`, `visible_slice()`,
  scroll clamping, horizontal offset. Rendering via ratatui widgets only here.
- Deps: `ratatui`, `crossterm`, `clap` (derive), `anyhow`.

## Components / UI
Picker:
- List of regular files in current dir, name + size, sorted alphabetically.
- Keys: Up/Down/j/k move, Enter open, q/Esc quit.
- Empty dir → "no files" message, q to quit.

Viewer:
- Body: text lines with right-aligned line numbers, current line highlight optional.
- Status bar line 1: filename, line cur/total, wrap ON/OFF.
- Status bar line 2 (hint): `↑↓ scroll PgUp/PgDn g/G top/bottom w wrap ←→ h-scroll q quit`.
- Keys: Up/Down/j/k, PageUp/PageDown, g/G, w toggle wrap,
  Left/Right for h-scroll when wrap OFF, q/Esc quit (Esc back to picker if
  launched from picker, else quit).

## Data flow
1. Parse `Option<PathBuf>` via clap.
2. If `Some(path)` → `read_to_string` (UTF-8 lossy with notice) → Viewer.
3. If `None` → `read_dir(cwd)` → filter regular files → Picker.
4. Picker Enter → load file → Viewer.
5. Event loop: crossterm key events → `App::handle_key` mutates scroll/selection
   → ratatui draw. Resize handled by ratatui (recompute wrap each frame).
6. Wrap ON: logical lines wrapped at `viewport_width - gutter`; vertical scroll
   over display lines but status shows logical line. Wrap OFF: truncate,
   horizontal scroll offset.

## Error handling
- All fallible fns return `anyhow::Result`.
- Terminal restore guaranteed via guard (`Drop` disables raw mode, leaves alt screen).
- Missing file / permission / read_dir fail → propagate via anyhow, print
  clean stderr message on exit (terminal restored by guard). No in-TUI error
  screen in v1 (deferred). No panics on user IO.
- Non-UTF8: `from_utf8_lossy`, status shows `[lossy UTF-8]`.
- Directories / non-regular files excluded from picker; explicit dir arg → error.

## Testing / quality
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
- Unit tests (no terminal needed): wrap function, scroll clamp, h-scroll clamp,
  picker sort/filter, status line format.
- Manual check: open small file, long-line file, empty file, missing file,
  no-arg in empty and non-empty dir, resize, wrap toggle.

## Non-goals
Search, go-to-line prompt, editing, diff, images, network, config files.
