# Main-as-orchestrator split — design

Date: 2026-09-27. Status: approved in chat (4 sections).

## 1. Intent

- Goal: split `src/main.rs` (~2000 lines) so `main` stays a thin
  orchestrator and future features land as new small modules.
- Approved scope: pure relocation of `main.rs` code + small `App` API
  cleanup (new methods so `input`/`ui` stop poking fields directly).
- Non-goals: no behavior change, no redesign of `App` state, no touch
  to char-indexing / highlight-fallback / preview-cap internals.

## 2. Current anatomy (`src/main.rs`)

- Bootstrap: `Cli` (:21), `TerminalGuard` (:28), `main()` loop (:45).
- `handle()` (:77, ~570 lines): picker branch (:83), viewer commenting
  (:148), pending text-object (:191), searching overlay (:227), viewer
  normal (:286: motions, counts, words, visual, search nav, comments,
  sidebar, quit).
- Paint: `paint_ranges` (:667) + search (:652) / selection (:658)
  wrappers.
- Status strings: `viewer_hint` (:694), `visual_tag` (:704),
  `comments_tag` (:712), `comment_prompt` (:719), `status_line` (:773),
  `picker_prompt_line` (:798), `picker_preview_visible` (:810).
- Sidebar: `sidebar_lines` (:727). Preview IO: `PREVIEW_MAX_BYTES`
  (:815) + `read_preview_lines` (:822).
- Render: `render()` dispatcher (:835) + picker pane (:839, ~150 lines)
  + viewer pane (:995, ~300 lines incl. status bar :1228).
- Tests: `handle_tests` + `status_tests` (incl. `render_smoke`).

Existing feature modules stay as-is: `app.rs` (state + motions +
search state + load), `viewer.rs` (wrap/scroll math), `search.rs`,
`picker.rs` (discover/filter), `select.rs`, `highlight/`.

## 3. Target tree (feature-split, staged)

- `main.rs` — `mod` decls + `Cli` use + `main()` loop only (~70 lines).
- `cli.rs` — `Cli` verbatim.
- `tui.rs` — `TerminalGuard` + event loop (`poll/draw/handle`,
  `viewport_h` update stays as-is for now).
- `input/mod.rs` — `handle()` dispatcher by `Mode` + Ctrl-C + `MAX_COUNT`.
- `input/picker.rs` — `Mode::Picker` branch verbatim.
- `input/viewer.rs` — whole `Mode::Viewer` chain in one file (~450
  lines). Split into `motion`/`visual`/`search` only after it exceeds
  ~500 lines (YAGNI gate).
- `ui/mod.rs` — `render()` dispatcher by `Mode`.
- `ui/picker.rs` — picker prompt + list window + preview pane layout.
- `ui/viewer.rs` — text panel + 50-col sidebar slot + 2-row status bar.
- `ui/status.rs` — pure builders + bottom-bar assembly (incl.
  commenting input rows).
- `ui/sidebar.rs` — `sidebar_lines` verbatim.
- `ui/paint.rs` — `paint_ranges` + both wrappers (shared by picker and
  viewer).
- `picker.rs` — absorbs file-domain helpers: `read_preview_lines`,
  `PREVIEW_MAX_BYTES`, `picker_prompt_line`, `picker_preview_visible`.

## 4. Data flow + invariants (do not change)

- Render order: slice styled spans → paint search (yellow) → paint
  selection (blue) → cursor cell wins (`main.rs:1089-1200`).
- Input priority: commenting → pending text-object → searching →
  normal. Esc priority: pending/visual → committed search → quit/back
  (`main.rs:287-308`).
- Wrap chunks for one logical line are consecutive; `chunk_k` counts
  repeats. Cursor/scroll coupling: `ensure_cursor_visible` then
  `clamp_scroll`; match recentering only outside margin 2.
- All text indexing stays char-based. `highlight_file` never drops
  text (`None` → `Span::raw` fallback). Preview read stays byte-capped
  and infallible.

## 5. `App` API cleanup (minimal, one method per step)

1. `App::open_selected_entry() -> Result<bool>` (picker Enter).
2. `App::commit_comment_draft()` / `App::cancel_commenting()`.
3. `App::step_search(dir)` (`n`/`N`/Enter-in-search via `find_matches`
   + `scroll_for_match`).
- Fields stay `pub` during the move; privatize one by one after each
  step is green. Existing `App::move_*`, `start/cancel/commit_search`,
  `ensure_cursor_visible`, `toggle_wrap`, `picker_*` stay untouched.

## 6. Error handling

- No new error paths. `main` keeps `anyhow::Result`; `load_file`
  context stays; preview path never fails (empty lines + note).

## 7. Testing

- All 62 tests move with their code, no behavior change:
  picker cases → `input/picker.rs`, viewer keys → `input/viewer.rs`,
  status/paint/sidebar/preview → `ui/*` + `picker.rs`,
  `render_smoke_*` → `ui/viewer.rs`.
- Gate after every move: `cargo test` + `cargo clippy -- -D warnings`.

## 8. Migration order (each step green)

1. `cli.rs` + `tui.rs`. 2. `ui/paint|status|sidebar`.
2. Preview helpers into `picker.rs`. 4. `ui/picker.rs` + `ui/viewer.rs`.
3. `input/*`. 6. `App` methods one by one. 7. Thin `main.rs`, final
   test + clippy.

## 9. Success criteria

- `main.rs` ~70 lines of wiring; every other file has one clear owner
  (screen or widget); new feature = new file.
- `cargo test` green, clippy clean, zero behavior diff (manual smoke:
  picker open, search `/ n N`, visual `v/V`, comment Enter, `C`
  sidebar, wrap Ctrl-W).
