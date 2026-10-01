# AGENTS.md

Single-file-ish Rust TUI text viewer (`ratatui` 0.29 + `crossterm` 0.28 + `clap` 4.5 derive). Binary crate, no workspace, no lib target.

## Commands

- `cargo run -- <path>` — open file directly; `cargo run` with no path opens picker for cwd.
- `cargo test` — 62 unit tests, all inline `#[cfg(test)]` modules (no `tests/` dir). Single test: `cargo test <name_substring>`.
- `cargo build --no-default-features --features lang-rust` — minimal-feature check; per-language features are `lang-<name>` in `Cargo.toml`.
- No rustfmt/clippy/CI config in repo; keep `cargo clippy -- -D warnings` clean anyway (`search.rs` uses `#![allow(dead_code)]` pattern for staged helpers).

## Architecture (`src/`)

- `main.rs` — everything TUI: `TerminalGuard` (raw mode + alt screen), event loop, `handle()` key dispatch, `render()`, `paint_search_ranges()`, `status_line()`. Key handling tests live here (`handle_tests`, `status_tests`).
- `app.rs` — `App` state (`Mode::Picker | Viewer`, cursor/scroll/wrap, search state with `saved_cursor`/`saved_scroll`, `lang` + `highlighted` spans, `theme`). `load_file()` reads bytes lossy-UTF8, sets `[lossy UTF-8]` note, detects theme/lang, pre-highlights whole file.
- `viewer.rs` — pure wrap/scroll math: `wrap_line` (char-based, never byte-slice), `build_display_lines`, `display_row_for_cursor`, `clamp_scroll`/`clamp_hscroll`, `content_size` (subtracts `Borders::ALL` + gutter).
- `search.rs` — `find_matches` (literal substring, smart-case: any uppercase → case-sensitive, char indices not bytes, `MAX_MATCHES=10_000` cap), `scroll_for_match` (margin 2), `chunk_match_ranges`.
- `picker.rs` — `list_files` (regular files only, sorted by name, skips dirs/errors).
- `highlight/` — `mod.rs` (`detect` by extension + Gemfile/Rakefile special-case, `highlight_file` → per-logical-line `Span`s, `slice_spans` char-window), `langs.rs` (per-language `LazyLock<HighlightConfiguration>` gated on `lang-*` features; `configuration()` returns `None` when off), `theme.rs` (`COLORFGBG` heuristic, Dark default; style index = `HIGHLIGHT_NAMES` order).

## Gotchas

- All text indexing is **char-based** (`chars().count()`, `chars().collect()`). Never use byte offsets except inside `highlight_file`'s tree-sitter `Source { start, end }` range, which is guarded with `text.get(start..end)`.
- `highlight_file` must never drop text: backend failure or `None` config → `None` (plain `Span::raw` fallback); trailing `\n` pops the extra empty line; `\r` stripped per chunk.
- Markdown (`Lang::Markdown`) intentionally has no grammar — `tree-sitter-markdown` 0.7.1 ships no highlight queries and links tree-sitter 0.19 symbols colliding with the 0.27 runtime. Keep plain fallback; see `Cargo.toml` comment.
- Render pipeline in `main.rs::render` is order-sensitive: slice styled spans → paint search ranges (yellow bg) → split cursor cell (cursor wins). Wrap chunks for one logical line are consecutive; `chunk_k` counts repeats.
- Cursor/scroll coupling: `ensure_cursor_visible(display_row)` then `clamp_scroll`; search-match recentering only when match is outside margin (`search::scroll_for_match` with margin 2).
- Design docs under `docs/superpowers/{specs,plans}/` are historical context only, not executable config.
