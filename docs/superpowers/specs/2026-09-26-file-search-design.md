# File Search with Viewport-Aware Jump — Design Spec
Date: 2026-09-26
Status: approved

## Goal
Add vim-style in-file search to `contextual` viewer: type a query, jump cursor to matches with `n`/`N`, without gratuitous scrolling. Scroll only when the match is outside the viewport or uncomfortably close to its edge.

## Scope (agreed)
- Trigger `/` in Viewer mode; status bar becomes search input with live preview.
- `Enter` commits (jump to first match at/after cursor), `Esc` cancels (restore pre-search cursor/scroll), `n`/`N` next/prev with wrap-around.
- Literal substring matching, smart-case (case-insensitive unless query contains uppercase).
- Viewport-aware scroll with 2-line margin (display-row based, wrap-aware).
- Match count + current-match highlight in viewport.
- No regex, no replace, no cross-file search, no symbol outline. YAGNI (deferred).

## Approach: A — In-place search state (chosen)
Rejected B (separate picker-style results list, double UI code for little v1 gain) and C (regex engine from day one, needs `regex` dep + invalid-pattern states).

## Architecture
- `src/app.rs` — extend `App`: `search_query: String`, `search_matches: Vec<(usize, usize)>` (logical line, char-col), `search_idx: usize`, `searching: bool`, saved cursor/scroll for cancel restore. Pure helpers: `find_matches(lines, query) -> Vec<...>`, `scroll_for_match(match_display_row, scroll, vh, margin) -> usize`.
- `src/viewer.rs` — reuse `display_row_for_cursor`, `clamp_scroll`, `build_display_lines`. No new wrapping logic.
- `src/main.rs` — `handle()` gains search-input branch (typing, Backspace, Enter, Esc); Viewer branch gains `/`, `n`, `N`. `render()` draws input in status bar + match highlights via `slice_spans` overlay.
- No new dependencies.

## Components / UI
Search input (status bar line 1 while `searching`):
- Format: `/query  3/12` or `/query  [no matches]`; cursor `|` at end (no mid-line editing in v1).
- Each keystroke recomputes matches live and previews nearest match (cursor follows, scroll rule applies).

Highlights:
- Current match: reverse/dark-gray bg (same as cursor style) so it reads as "cursor is here".
- Other matches in viewport: subtle underline. Overlay composes over tree-sitter spans; highlight underneath preserved.

Keys (Viewer):
- `/` enter search, `Esc` cancel/restore, `Enter` commit, `n` next, `N` / `Shift+N` prev. `n`/`N` with no active query or no matches: no-op.

## Data flow
1. `/` → save cursor/scroll, `searching=true`, clear query/matches.
2. Keystroke → update `search_query` → `find_matches` (char-based literal scan, smart-case folded) → set `search_idx` to nearest match at/after cursor for preview → set cursor to match → compute match display row → apply scroll rule → redraw.
3. `Enter` → `searching=false`, keep cursor/scroll plus retained `search_query/matches/idx` for `n`/`N`. `Esc` → `searching=false`, restore saved cursor/scroll, clear search state.
4. `n`/`N` (when not searching, last committed query present) → `idx = (idx ± 1) mod len` → same cursor + scroll-rule step.

## Viewport scroll rule
- All math in display rows (wrap-expanded), via `display_row_for_cursor`.
- `margin = 2`; effective margin `m = if vh < 2*margin+1 { 0 } else { margin }` (so `m=0` when `vh<5`); `vh==0` leaves scroll untouched.
- No scroll if match row in `[scroll+margin, scroll+vh-1-margin]`.
- Else minimal scroll: above/near-top → `scroll = match_row - margin`; below/near-bottom → `scroll = match_row - vh + 1 + margin`; then `clamp_scroll`.
- Horizontal (`h_scroll`) unchanged: existing follow-cursor logic applies; search adds nothing.
- Example: `vh=20, scroll=0`, match row 18 → near bottom → `scroll = 18-20+1+2 = 1`. Match row 10 → untouched.

## Error handling
- Empty query: no matches, no jump, stays in input.
- No matches: status shows `[no matches]`, `Enter` exits input keeping position, `n`/`N` no-op.
- Unicode: all cols are char indices, never byte offsets; slicing via chars.
- Large files: linear scan per keystroke is acceptable for viewer sizes; cap stored matches at ~10k (status shows `10k+`) to bound memory.

## Testing / quality
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
- Unit: matcher (literal, smart-case upper/lower, unicode, overlapping, empty query), scroll-margin fn (middle untouched / near-edge scrolls / outside scrolls / tiny viewport saturates / clamp at EOF), `n`/`N` wrap-around.
- Manual: search in wrapped vs unwrapped file, match at top/bottom edges, no-match query, Esc restore, resize mid-search.

## Non-goals
Regex/ignore-case toggles, replace, persistent search history, jump-label (flash) mode, symbol search, multi-file search.
