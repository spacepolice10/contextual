# Tree-sitter Syntax Highlighting — Design Spec
Date: 2026-09-24
Status: approved

## Goal
Add read-only syntax highlighting to the `contextual` terminal viewer using
tree-sitter (raw `tree-sitter-highlight`, the Zed/Helix pattern). No editing,
no config in v1. Ships with popular languages, decoupled from viewer logic.

## Scope (agreed)
- Languages v1 (by extension): ruby (`rb`, `erb`, `Gemfile`/`Rakefile` by
  filename), js (`js`, `jsx`), ts (`ts`), tsx (`tsx`), html (`html`, `htm`,
  `erb` inner HTML not split — whole file as ruby per `erb` rule above),
  elixir (`ex`, `exs`), php (`php`), c (`c`, `h`), plus base: rust (`rs`),
  python (`py`), toml (`toml`, via `tree-sitter-toml-ng`). Markdown (`md`)
  renders plain in v1: upstream `tree-sitter-markdown` ships no queries and
  links an incompatible runtime (follow-up when that changes).
- Unknown extension → plain text render (current behavior). This is normal,
  not an error.
- Theme: bundled dark + light pair. Selection: `COLORFGBG` env heuristic when
  present, else Dark. (OSC 11 query deferred: it needs nonblocking stdin reads
  the std lib can't do portably — follow-up, not v1.) No user config in v1.
- Cursor highlight paints on top of syntax spans (existing cursor logic).
- Wrap works on highlighted spans (split by char boundary, styles inherited).
- Deferred: runtime `.so` grammar plugins (documented as future work),
  user-supplied themes, per-file language override flag.

## Approach: A — raw tree-sitter-highlight (chosen)
Rejected B (wrapper crates `syntastica`/`autumnus`: less boilerplate but
third-party theme/version decisions plus maturity risk) and C (`bat` as lib:
fastest but syntect, not the tree-sitter standard the project chose).

## Architecture
New module `src/highlight/`, nothing else restructured:
- `src/highlight/mod.rs` — public interface only:
  - `detect(path: &Path) -> Option<Lang>` (extension + special filenames)
  - `highlight_file(text: &str, lang: Lang, theme: Theme) -> Vec<Vec<Span<'static>>>`
    (one entry per logical line; file-level pass so multi-line constructs
    like block comments highlight correctly; caller slices the viewport)
  - `enum Lang`, `enum Theme { Dark, Light }`
- `src/highlight/langs.rs` — per-language `HighlightConfiguration`
  (queries taken from each grammar repo's `queries/` dir), built once behind
  `LazyLock`; registry `match Lang -> &Configuration`. Adding a language =
  one grammar dep + ~10 lines here.
- `src/highlight/theme.rs` — capture-name → `Style` maps for both themes
  (`keyword`, `string`, `comment`, `type`, `function`, `variable`, …).
- `main.rs` / `viewer.rs` never import tree-sitter types. `App` gains
  `lang: Option<Lang>` plus `highlighted: Option<Vec<Vec<Span>>>` (per-logical-line
  spans), both computed once in `App::load_file`. Render slices the viewport
  out of `highlighted` when `Some`, else uses the current plain path.
  Cursor split applies after, on the target display row.
- Grammar crates behind Cargo features (`lang-ruby`, `lang-javascript`,
  … plus `all-langs`); default = all v1 languages on.

## Components
- Detector: pure `Path -> Option<Lang>`, unit-testable, no tree-sitter dep.
- Highlighter: owns `Highlighter` + configurations + `HighlightState`;
  converts `HighlightEvent::{Source, HighlightStart/End}` byte ranges into
  ratatui `Span`s with theme styles.
- Theme maps: two `&[(&str, Style)]` tables (dark/light); unknown capture
  names fall back to default text style (never panic, never drop text).
- Viewer integration: ~20 lines in `render()` (branch on `app.lang`,
  split highlighted spans at wrap width / h-scroll window, cursor overlay
  unchanged in behavior).

## Data flow
1. `App::load_file` → `highlight::detect(path)` → store `Option<Lang>`; if
   `Some`, run `highlight::highlight_file` once and store per-line spans
   (read-only file, so no cache invalidation ever).
2. Render: slice visible logical lines out of `highlighted` (or plain `lines`
   when `None`); wrap-split the spans char-safely at viewport width (wrap ON)
   or cut the h-scroll window (wrap OFF) → cursor cell split on cursor row
   → `Line::from(spans)`.
3. Resize/re-render re-slices the stored spans; no recompute per frame.

## Error handling
- Grammar build / query load failure at startup of a language → fall back to
  plain for files of that language + status-bar note `[no highlight]`.
  Never panic, never drop file content.
- Unknown capture name → default style. Malformed line bytes → lossy char
  handling (file already loaded lossy upstream).
- All fallible init returns `anyhow::Result`; terminal guard behavior
  unchanged.

## Testing / quality
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
- Unit tests (no terminal): `detect` for every mapped extension + special
  filenames + unknown → `None`; `highlight_file` returns styled spans for
  a snippet of each v1 language (incl. a multi-line block comment proving
  file-level state); fallback test for unknown language;
  wrap-split preserves all chars (concat of chunks == source line).
- Manual: open one file per language, toggle `w`, move cursor over
  highlighted tokens, resize, check light/dark auto-detect.
- Perf note: highlight runs once per file load, O(file); render is O(viewport).
  If a 10k+ line file loads slowly, follow-up is lazy per-line highlighting
  (design leaves room: it lives inside `highlight/`, interface unchanged).

## Non-goals
Runtime grammar plugins, user themes, language override CLI flag, editing,
semantic tokens (LSP), diff-aware highlight.
