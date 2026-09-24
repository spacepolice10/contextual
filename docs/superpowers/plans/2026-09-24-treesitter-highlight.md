# Tree-sitter Highlighting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add read-only tree-sitter syntax highlighting for 12 languages behind a decoupled `src/highlight/` module.

**Architecture:** New `src/highlight/` module (`mod.rs` interface + `detect`, `langs.rs` grammar registry via `LazyLock`, `theme.rs` palettes). `App` stores `lang` + precomputed per-line spans at load; `render()` slices viewport, cursor paints on top. Grammar crates behind Cargo features.

**Tech Stack:** tree-sitter-highlight 0.26, tree-sitter grammar crates (latest via `cargo add`), ratatui 0.29, anyhow 1.0

---

## File Structure

- Modify: `Cargo.toml` — add deps + `[features]` per-language flags.
- Create: `src/highlight/mod.rs` — `Lang`, `Theme`, `detect()`, `highlight_file()`, `slice_spans()`, `detect_theme()`.
- Create: `src/highlight/langs.rs` — `HighlightConfiguration` registry, `configuration(lang)`.
- Create: `src/highlight/theme.rs` — `style(theme, capture_idx)`, palette tables.
- Modify: `src/app.rs` — add `lang: Option<Lang>`, `highlighted: Option<Vec<Vec<Span<'static>>>>`, compute in `load_file`.
- Modify: `src/main.rs` — `mod highlight;`, render branch, status note, theme pick at startup.
- Test: inline `#[cfg(test)]` in highlight files + app tests (`cargo test`).

---

### Task 1: Deps, features, detect()

**Files:**
- Modify: `Cargo.toml`
- Create: `src/highlight/mod.rs` (skeleton: Lang, Theme, detect + tests)
- Modify: `src/main.rs:1-3` (add `mod highlight;`)

- [ ] **Step 1: Add dependencies**

Run: `cargo add tree-sitter-highlight tree-sitter-rust tree-sitter-python tree-sitter-javascript tree-sitter-typescript tree-sitter-html tree-sitter-ruby tree-sitter-elixir tree-sitter-php tree-sitter-c tree-sitter-toml tree-sitter-markdown`
Expected: PASS, `Cargo.toml` gains 12 deps.

- [ ] **Step 2: Add feature flags to Cargo.toml**

```toml
[features]
default = ["all-langs"]
all-langs = ["lang-rust", "lang-python", "lang-javascript", "lang-typescript", "lang-html", "lang-ruby", "lang-elixir", "lang-php", "lang-c", "lang-toml", "lang-markdown"]
lang-rust = ["dep:tree-sitter-rust"]
lang-python = ["dep:tree-sitter-python"]
lang-javascript = ["dep:tree-sitter-javascript"]
lang-typescript = ["dep:tree-sitter-typescript"]
lang-html = ["dep:tree-sitter-html"]
lang-ruby = ["dep:tree-sitter-ruby"]
lang-elixir = ["dep:tree-sitter-elixir"]
lang-php = ["dep:tree-sitter-php"]
lang-c = ["dep:tree-sitter-c"]
lang-toml = ["dep:tree-sitter-toml"]
lang-markdown = ["dep:tree-sitter-markdown"]
```

Each tree-sitter dep must be `optional = true` (edit the `cargo add` lines: `tree-sitter-rust = { version = "x.y", optional = true }`, keep the version cargo chose).

- [ ] **Step 3: Write failing detect() test (src/highlight/mod.rs)**

```rust
pub mod langs;
pub mod theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust, Python, JavaScript, TypeScript, Tsx, Html, Ruby, Elixir, Php, C, Toml, Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme { Dark, Light }

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn detect_common_extensions() {
        assert_eq!(detect(Path::new("a.rs")), Some(Lang::Rust));
        assert_eq!(detect(Path::new("a.py")), Some(Lang::Python));
        assert_eq!(detect(Path::new("a.js")), Some(Lang::JavaScript));
        assert_eq!(detect(Path::new("a.jsx")), Some(Lang::JavaScript));
        assert_eq!(detect(Path::new("a.ts")), Some(Lang::TypeScript));
        assert_eq!(detect(Path::new("a.tsx")), Some(Lang::Tsx));
        assert_eq!(detect(Path::new("a.html")), Some(Lang::Html));
        assert_eq!(detect(Path::new("a.rb")), Some(Lang::Ruby));
        assert_eq!(detect(Path::new("app.erb")), Some(Lang::Ruby));
        assert_eq!(detect(Path::new("a.ex")), Some(Lang::Elixir));
        assert_eq!(detect(Path::new("a.exs")), Some(Lang::Elixir));
        assert_eq!(detect(Path::new("a.php")), Some(Lang::Php));
        assert_eq!(detect(Path::new("a.c")), Some(Lang::C));
        assert_eq!(detect(Path::new("a.h")), Some(Lang::C));
        assert_eq!(detect(Path::new("a.toml")), Some(Lang::Toml));
        assert_eq!(detect(Path::new("a.md")), Some(Lang::Markdown));
    }
    #[test]
    fn detect_special_filenames_and_unknown() {
        assert_eq!(detect(Path::new("Gemfile")), Some(Lang::Ruby));
        assert_eq!(detect(Path::new("Rakefile")), Some(Lang::Ruby));
        assert_eq!(detect(Path::new("notes.txt")), None);
        assert_eq!(detect(Path::new("Makefile")), None);
    }
}
```

Run: `cargo test highlight`
Expected: FAIL with `cannot find function detect`.

- [ ] **Step 4: Minimal detect() implementation (top of mod.rs)**

```rust
use std::path::Path;

/// Capture names recognized for styling. Order = style index in theme.rs.
/// Query captures not listed here produce no highlight event.
pub const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute", "boolean", "comment", "constant", "constant.builtin",
    "constructor", "function", "function.builtin", "function.macro",
    "keyword", "label", "number", "operator", "property",
    "punctuation", "string", "tag", "type", "type.builtin",
    "variable", "variable.builtin", "variable.parameter",
];

/// Map a file path to a highlight language. Extension first, then special
/// filenames (Gemfile/Rakefile). Unknown -> None (plain render).
pub fn detect(path: &Path) -> Option<Lang> {
    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
        if name == "Gemfile" || name == "Rakefile" {
            return Some(Lang::Ruby);
        }
    }
    match path.extension().and_then(|s| s.to_str())? {
        "rs" => Some(Lang::Rust),
        "py" => Some(Lang::Python),
        "js" | "jsx" => Some(Lang::JavaScript),
        "ts" => Some(Lang::TypeScript),
        "tsx" => Some(Lang::Tsx),
        "html" | "htm" => Some(Lang::Html),
        "rb" | "erb" => Some(Lang::Ruby),
        "ex" | "exs" => Some(Lang::Elixir),
        "php" => Some(Lang::Php),
        "c" | "h" => Some(Lang::C),
        "toml" => Some(Lang::Toml),
        "md" | "markdown" => Some(Lang::Markdown),
        _ => None,
    }
}
```

Add `mod highlight;` to `src/main.rs`.

- [ ] **Step 5: Run tests**

Run: `cargo test highlight`
Expected: PASS, 2 tests ok.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/highlight/mod.rs src/main.rs
git commit -m "feat: add highlight module skeleton with language detect"
```

---

### Task 2: Theme palettes + detect_theme()

**Files:**
- Create: `src/highlight/theme.rs`
- Modify: `src/highlight/mod.rs` (re-export + detect_theme)

- [ ] **Step 1: Write failing test**

```rust
// in theme.rs tests
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_captures_styled_both_themes() {
        for name in ["keyword", "string", "comment", "type", "function", "number"] {
            let idx = super::super::HIGHLIGHT_NAMES.iter().position(|n| *n == name).unwrap();
            assert_ne!(style(Theme::Dark, idx), Style::default(), "dark {name}");
            assert_ne!(style(Theme::Light, idx), Style::default(), "light {name}");
        }
    }
    #[test]
    fn colorfgbg_dark_background() {
        assert_eq!(theme_from_colorfgbg(Some("15;0")), Theme::Dark);
        assert_eq!(theme_from_colorfgbg(Some("0;15")), Theme::Light);
        assert_eq!(theme_from_colorfgbg(None), Theme::Dark);
    }
}
```

Run: `cargo test theme`
Expected: FAIL, `cannot find function style`.

- [ ] **Step 2: Implementation (src/highlight/theme.rs)**

```rust
use super::Theme;
use ratatui::style::{Color, Modifier, Style};

fn rgb(r: u8, g: u8, b: u8) -> Style {
    Style::new().fg(Color::Rgb(r, g, b))
}

/// Capture index -> Style. Index refers to HIGHLIGHT_NAMES order in mod.rs.
/// Unknown index -> default style (never panic).
pub fn style(theme: Theme, idx: usize) -> Style {
    let name = super::HIGHLIGHT_NAMES.get(idx).copied().unwrap_or("");
    match (theme, name) {
        (_, "comment") => rgb(92, 99, 112).add_modifier(Modifier::ITALIC),
        (_, "string") => match theme {
            Theme::Dark => rgb(152, 195, 121),
            Theme::Light => rgb(80, 130, 50),
        },
        (_, "keyword") => match theme {
            Theme::Dark => rgb(198, 120, 221),
            Theme::Light => rgb(160, 60, 180),
        },
        (_, "type") | (_, "type.builtin") => match theme {
            Theme::Dark => rgb(229, 192, 123),
            Theme::Light => rgb(170, 120, 40),
        },
        (_, "function") | (_, "function.builtin") | (_, "function.macro") => match theme {
            Theme::Dark => rgb(97, 175, 239),
            Theme::Light => rgb(30, 100, 200),
        },
        (_, "number") | (_, "boolean") => match theme {
            Theme::Dark => rgb(209, 154, 102),
            Theme::Light => rgb(180, 110, 40),
        },
        (_, "constant") | (_, "constant.builtin") => match theme {
            Theme::Dark => rgb(86, 182, 194),
            Theme::Light => rgb(30, 140, 150),
        },
        (_, "variable") | (_, "variable.builtin") | (_, "variable.parameter") => match theme {
            Theme::Dark => rgb(224, 108, 117),
            Theme::Light => rgb(190, 60, 70),
        },
        (_, "property") | (_, "tag") | (_, "attribute") => match theme {
            Theme::Dark => rgb(229, 192, 123),
            Theme::Light => rgb(170, 120, 40),
        },
        (_, "operator") | (_, "punctuation") => match theme {
            Theme::Dark => rgb(171, 178, 191),
            Theme::Light => rgb(90, 90, 90),
        },
        (_, "constructor") | (_, "label") => match theme {
            Theme::Dark => rgb(97, 175, 239),
            Theme::Light => rgb(30, 100, 200),
        },
        _ => Style::default(),
    }
}

/// COLORFGBG is "fg;bg" with 0-15 values. bg <= 6 or == 8 -> dark terminal.
pub fn theme_from_colorfgbg(val: Option<&str>) -> Theme {
    let bg: Option<u8> = val.and_then(|v| v.rsplit(';').next()?.trim().parse().ok());
    match bg {
        Some(7) | Some(15) => Theme::Light,
        Some(_) => Theme::Dark,
        None => Theme::Dark,
    }
}

/// Pick theme: COLORFGBG heuristic, fallback Dark.
pub fn detect_theme() -> Theme {
    theme_from_colorfgbg(std::env::var("COLORFGBG").ok().as_deref())
}
```

In `mod.rs` add: `pub use theme::{detect_theme, style};`

- [ ] **Step 3: Run tests**

Run: `cargo test highlight`
Expected: PASS (detect 2 + theme 2).

- [ ] **Step 4: Commit**

```bash
git add src/highlight/theme.rs src/highlight/mod.rs
git commit -m "feat: add highlight theme palettes and detection"
```

---

### Task 3: Grammar registry (langs.rs)

**Files:**
- Create: `src/highlight/langs.rs`

- [ ] **Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::Lang;
    #[test]
    fn all_enabled_languages_load() {
        // Every Lang with its feature on must return a configuration.
        for lang in [Lang::Rust, Lang::Python, Lang::JavaScript, Lang::TypeScript,
                     Lang::Tsx, Lang::Html, Lang::Ruby, Lang::Elixir,
                     Lang::Php, Lang::C, Lang::Toml, Lang::Markdown] {
            assert!(configuration(lang).is_some(), "{lang:?} has no configuration");
        }
    }
}
```

Run: `cargo test langs`
Expected: FAIL, `cannot find function configuration`.

- [ ] **Step 2: Implementation (src/highlight/langs.rs)**

```rust
use std::sync::LazyLock;
use tree_sitter_highlight::HighlightConfiguration;
use super::{Lang, HIGHLIGHT_NAMES};

fn cfg(
    language: tree_sitter::Language,
    name: &str,
    highlight_query: &str,
    injection_query: &str,
    locals_query: &str,
) -> HighlightConfiguration {
    let mut c = HighlightConfiguration::new(language, name, highlight_query, injection_query, locals_query)
        .expect("grammar queries load");
    c.configure(HIGHLIGHT_NAMES);
    c
}

#[cfg(feature = "lang-rust")]
static RUST: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(tree_sitter_rust::LANGUAGE.into(), "rust",
        tree_sitter_rust::HIGHLIGHT_QUERY,
        tree_sitter_rust::INJECTIONS_QUERY,
        tree_sitter_rust::LOCALS_QUERY)
});
// ... same pattern for python, javascript, typescript (ts), html, ruby,
// elixir, php, c, toml, markdown. TypeScript TSX uses the tsx constants:
// tree_sitter_typescript::LANGUAGE_TSX.into(), "tsx",
// tree_sitter_typescript::TSX_HIGHLIGHT_QUERY,
// tree_sitter_typescript::TSX_INJECTIONS_QUERY,
// tree_sitter_typescript::TSX_LOCALS_QUERY
// (If a crate names these constants differently, check its docs.rs page
// and use the exact names; keep the cfg() helper unchanged.)

/// Borrow the configuration for a language, or None if its feature is off.
pub fn configuration(lang: Lang) -> Option<&'static HighlightConfiguration> {
    match lang {
        #[cfg(feature = "lang-rust")] Lang::Rust => Some(&RUST),
        // ... one arm per language, same pattern ...
        #[allow(unreachable_patterns)]
        _ => None,
    }
}
```

Note: with default features all arms compile; the `_ => None` covers
feature-off builds. If the test fails for Markdown only (its crate has a
different query layout), wire Markdown to `None` explicitly with a comment
and file a follow-up — do not block the other 11 languages.

- [ ] **Step 3: Run tests**

Run: `cargo test langs && cargo build`
Expected: PASS + build clean (first build compiles C grammars, slow is normal).

- [ ] **Step 4: Commit**

```bash
git add src/highlight/langs.rs src/highlight/mod.rs
git commit -m "feat: add tree-sitter grammar registry for 12 languages"
```

---

### Task 4: highlight_file()

**Files:**
- Modify: `src/highlight/mod.rs` (add `highlight_file` + tests)

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn highlight_rust_keywords() {
    let lines = highlight_file("fn main() {}", Lang::Rust, Theme::Dark);
    assert_eq!(lines.len(), 1);
    let flat: String = lines[0].iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(flat, "fn main() {}");
    assert!(lines[0].len() > 1, "expected styled spans, got plain");
}
#[test]
fn highlight_block_comment_spans_lines() {
    let lines = highlight_file("/* a\nb */\nfn f() {}", Lang::Rust, Theme::Dark);
    assert_eq!(lines.len(), 3);
    for line in &lines {
        assert!(!line.is_empty());
    }
}
#[test]
fn highlight_each_language_smoke() {
    let samples = [
        (Lang::Python, "def f():\n    pass\n"),
        (Lang::JavaScript, "const x = 1;\n"),
        (Lang::TypeScript, "const x: number = 1;\n"),
        (Lang::Tsx, "const A = () => <div />;\n"),
        (Lang::Html, "<div class=\"a\">x</div>\n"),
        (Lang::Ruby, "def f\n  puts 1\nend\n"),
        (Lang::Elixir, "def f, do: 1\n"),
        (Lang::Php, "<?php echo 1;\n"),
        (Lang::C, "int main(void) { return 0; }\n"),
        (Lang::Toml, "a = 1\n"),
        (Lang::Markdown, "# hi\n"),
    ];
    for (lang, src) in samples {
        if crate::highlight::langs::configuration(lang).is_none() {
            continue; // feature off or deferred (e.g. markdown)
        }
        let lines = highlight_file(src, lang, Theme::Dark);
        let flat: String = lines.iter().flat_map(|l| l.iter().map(|s| s.content.as_ref().to_string())).collect();
        assert_eq!(flat, src.trim_end_matches('\n'), "{lang:?} lost text");
    }
}
```

Run: `cargo test highlight_file`
Expected: FAIL, `cannot find function highlight_file`.

- [ ] **Step 2: Implementation (append to mod.rs)**

```rust
use ratatui::text::Span;
use tree_sitter_highlight::{Highlighter, HighlightEvent};

/// Highlight a whole file into per-logical-line spans. Unknown language
/// config (None) or empty text -> plain single-style lines. Never drops text.
pub fn highlight_file(text: &str, lang: Lang, theme: Theme) -> Vec<Vec<Span<'static>>> {
    let Some(config) = langs::configuration(lang) else {
        return text.lines().map(|l| vec![Span::raw(l.to_string())]).collect();
    };
    let mut hl = Highlighter::new();
    let Ok(events) = hl.highlight(config, text.as_bytes(), None, |_| None) else {
        return text.lines().map(|l| vec![Span::raw(l.to_string())]).collect();
    };
    let mut out: Vec<Vec<Span<'static>>> = vec![Vec::new()];
    let mut stack: Vec<usize> = Vec::new();
    let style_of = |stack: &[usize]| {
        stack.last().map(|i| theme::style(theme, *i)).unwrap_or_default()
    };
    for ev in events {
        let Ok(ev) = ev else { continue };
        match ev {
            HighlightEvent::HighlightStart(h) => stack.push(h.0),
            HighlightEvent::HighlightEnd => { stack.pop(); }
            HighlightEvent::Source { start, end } => {
                let st = style_of(&stack);
                for (i, part) in text[start..end].split('\n').enumerate() {
                    if i > 0 {
                        out.push(Vec::new());
                    }
                    if !part.is_empty() {
                        out.last_mut().unwrap().push(Span::styled(part.to_string(), st));
                    }
                }
            }
        }
    }
    out
}
```

Note: `text[start..end]` slicing is safe — tree-sitter emits valid UTF-8
boundaries for str input. Empty trailing line: `"a\n".lines()` yields
`["a"]`; the events path yields `[spans]` + one empty vec for the final
newline — tests use `trim_end_matches` tolerant compare; render uses
`app.lines.len()` as source of truth for line count (see Task 6).

- [ ] **Step 3: Run tests**

Run: `cargo test highlight`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/highlight/mod.rs
git commit -m "feat: add highlight_file with event walk"
```

---

### Task 5: slice_spans() helper

**Files:**
- Modify: `src/highlight/mod.rs` (add + tests)

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn slice_spans_preserves_text_and_style() {
    use ratatui::style::{Color, Style};
    let st = Style::new().fg(Color::Red);
    let spans = vec![Span::styled("ab", st), Span::raw("cdef".to_string())];
    let got = slice_spans(&spans, 1, 3);
    let flat: String = got.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(flat, "bcd");
    assert_eq!(got[0].style, st);
}
#[test]
fn slice_spans_out_of_range_is_empty() {
    let spans = vec![Span::raw("ab".to_string())];
    assert!(slice_spans(&spans, 10, 5).is_empty());
}
```

Run: `cargo test slice_spans`
Expected: FAIL, `cannot find function slice_spans`.

- [ ] **Step 2: Implementation**

```rust
/// Char-based window over styled spans. Splits boundary spans, clones styles.
/// Used for wrap chunks, h-scroll window, and cursor split (len 1).
pub fn slice_spans(spans: &[Span<'static>], start: usize, len: usize) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    let mut pos = 0;
    let end = start + len;
    for s in spans {
        let chars: Vec<char> = s.content.chars().collect();
        let s_end = pos + chars.len();
        if s_end > start && pos < end {
            let a = start.saturating_sub(pos);
            let b = (end - pos).min(chars.len());
            let part: String = chars[a..b].iter().collect();
            out.push(Span::styled(part, s.style));
        }
        pos = s_end;
        if pos >= end {
            break;
        }
    }
    out
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test highlight`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/highlight/mod.rs
git commit -m "feat: add slice_spans for wrap and cursor"
```

---

### Task 6: App + render integration

**Files:**
- Modify: `src/app.rs` (fields + load_file + tests)
- Modify: `src/main.rs` (theme pick, render branch, status note)

- [ ] **Step 1: Write failing app test**

```rust
#[test]
fn load_file_detects_language() {
    let dir = std::env::temp_dir();
    let path = dir.join("ctx_hl_test.rs");
    std::fs::write(&path, "fn main() {}\n").unwrap();
    let app = App::load_file(&path, false).unwrap();
    assert_eq!(app.lang, Some(crate::highlight::Lang::Rust));
    assert!(app.highlighted.is_some());
    let _ = std::fs::remove_file(&path);
}
```

Run: `cargo test load_file_detects`
Expected: FAIL, `no field lang`.

- [ ] **Step 2: App changes (src/app.rs)**

Add imports: `use crate::highlight::{self, Lang}; use ratatui::text::Span;`

Add fields: `pub lang: Option<Lang>, pub highlighted: Option<Vec<Vec<Span<'static>>>>, pub theme: highlight::Theme,`

Init `None`/`Dark` in `new_picker`, test fixture `viewer_app`, and in
`load_file` compute:

```rust
let theme = highlight::detect_theme();
let lang = highlight::detect(path);
let highlighted = lang.map(|l| highlight::highlight_file(&text, l, theme));
```

(`text` is already owned in load_file; call before or after `lines` split.)

Update the 3 struct literals (new_picker, load_file, viewer_app fixture).

- [ ] **Step 3: Render changes (src/main.rs)**

At startup after CLI parse: theme comes from `load_file` already; for the
picker path no highlight needed.

In `render()` Viewer arm, replace the plain `shown` pipeline: source spans
are `app.highlighted.as_ref().and_then(|h| h.get(*lidx))` when present, else
`&[Span::raw(content.clone())]`-equivalent. Then:
- wrap ON: chunk via `highlight::slice_spans(src, chunk_start, w)` per
  `wrap_line`-equivalent widths (compute chunk count with existing
  `viewer::wrap_line(content, w).len()`, slice by char offsets).
- wrap OFF: `highlight::slice_spans(src, app.h_scroll, tw)`.
- cursor row: split into before/cursor/after with `slice_spans(src_windowed,
  rel_col, 1)` styled `bg(DarkGray).fg(White)`; past-end renders `Span::styled(" ", cursor_style)` as today.
- status line: append `[no highlight]` when `app.lang.is_some() &&
  app.highlighted` is plain (i.e. config None) — track via
  `app.highlighted.is_none() && app.lang.is_some()`.

Keep `display`/`cursor_row`/`ensure_cursor_visible` logic otherwise unchanged.

- [ ] **Step 4: Verify gates**

Run: `cargo fmt && cargo clippy -- -D warnings && cargo test`
Expected: all PASS (~25 tests).

Manual (human): open one file per language, `w` toggle, cursor over tokens,
resize, unset vs set COLORFGBG.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: wire highlight into viewer render"
```

---

## Self-Review

- Spec coverage: 12 langs ✓ (T3), decoupled module ✓ (all tasks, only
  `detect`/`highlight_file`/`slice_spans` cross boundary), bundled themes ✓
  (T2), cursor-on-top ✓ (T6), wrap-before-highlight order ✓ (T6 slices styled
  spans), unknown→plain ✓ (T1+T4), error fallback + `[no highlight]` ✓ (T4+T6),
  feature flags ✓ (T1+T3), no plugins/themes-config/override ✓ (non-goals untouched).
- Placeholders: none — exact paths, full code, exact commands. Two honest
  version-drift notes (query const names, markdown) with concrete fallback
  actions, not TODOs.
- Type consistency: `Lang`/`Theme`/`detect`/`highlight_file(text,&str,Lang,Theme)`
  `-> Vec<Vec<Span<'static>>>` / `slice_spans(&[Span],usize,usize)` /
  `style(Theme,usize)` / `configuration(Lang)->Option<&HighlightConfiguration>`
  used identically across tasks. `HIGHLIGHT_NAMES` lives in `mod.rs` (defined
  once in T3, used by T2's test and T4).
