pub mod langs;
pub mod theme;

pub use theme::detect_theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Html,
    Ruby,
    Elixir,
    Php,
    C,
    Toml,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

use ratatui::text::Span;
use std::path::Path;
use tree_sitter_highlight::{HighlightEvent, Highlighter};

/// Capture names recognized for styling. Order = style index in theme.rs.
/// Query captures not listed here produce no highlight event.
pub const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "boolean",
    "comment",
    "constant",
    "constant.builtin",
    "constructor",
    "function",
    "function.builtin",
    "function.macro",
    "keyword",
    "label",
    "number",
    "operator",
    "property",
    "punctuation",
    "string",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

/// Map a file path to a highlight language. Special filenames first, then extension
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

/// Highlight a whole file into per-logical-line spans. Unknown language
/// config (None) or backend failure -> None (plain render via Span::raw).
/// Never drops text when Some.
///
/// Blank lines yield empty vecs (renderable via `Line::from(vec![])`);
/// nested highlights use top-of-stack style.
pub fn highlight_file(text: &str, lang: Lang, theme: Theme) -> Option<Vec<Vec<Span<'static>>>> {
    let config = langs::configuration(lang)?;
    let mut hl = Highlighter::new();
    let Ok(events) = hl.highlight(config, text.as_bytes(), None, None, |_| None) else {
        return None;
    };
    let mut out: Vec<Vec<Span<'static>>> = vec![Vec::new()];
    let mut stack: Vec<usize> = Vec::new();
    let style_of = |stack: &[usize]| {
        stack
            .last()
            .map(|i| theme::style(theme, *i))
            .unwrap_or_default()
    };
    for ev in events {
        let Ok(ev) = ev else { continue };
        match ev {
            HighlightEvent::HighlightStart(h) => stack.push(h.0),
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
            HighlightEvent::Source { start, end } => {
                let Some(chunk) = text.get(start..end) else {
                    continue;
                };
                let st = style_of(&stack);
                for (i, part) in chunk.split('\n').enumerate() {
                    if i > 0 {
                        out.push(Vec::new());
                    }
                    let part = part.strip_suffix('\r').unwrap_or(part);
                    if !part.is_empty() {
                        out.last_mut()
                            .unwrap()
                            .push(Span::styled(part.to_string(), st));
                    }
                }
            }
        }
    }
    if text.ends_with('\n') && out.last().is_some_and(Vec::is_empty) {
        out.pop();
    }
    Some(out)
}

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
    #[test]
    fn highlight_rust_keywords() {
        let lines =
            highlight_file("fn main() {}", Lang::Rust, Theme::Dark).expect("rust highlights");
        assert_eq!(lines.len(), 1);
        let flat: String = lines[0].iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(flat, "fn main() {}");
        assert!(lines[0].len() > 1, "expected styled spans, got plain");
    }
    #[test]
    fn highlight_block_comment_spans_lines() {
        let lines = highlight_file("/* a\nb */\nfn f() {}", Lang::Rust, Theme::Dark)
            .expect("rust highlights");
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
            let lines = highlight_file(src, lang, Theme::Dark).expect("config present => Some");
            let flat: String = lines
                .iter()
                .map(|l| {
                    l.iter()
                        .map(|s| s.content.as_ref().to_string())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(
                flat.trim_end_matches('\n'),
                src.trim_end_matches('\n'),
                "{lang:?} lost text"
            );
        }
    }
    #[test]
    fn highlight_trailing_newline_no_extra_line() {
        let lines =
            highlight_file("fn f() {}\n", Lang::Rust, Theme::Dark).expect("rust highlights");
        assert_eq!(lines.len(), 1);
    }
    #[test]
    fn highlight_multibyte_no_panic_flat_matches_source() {
        let src = "// héllo wörld 🌍\nfn f() {}";
        let lines = highlight_file(src, Lang::Rust, Theme::Dark).expect("rust highlights");
        let flat: String = lines
            .iter()
            .map(|l| l.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(flat, src);
    }
    #[test]
    fn highlight_crlf_matches_lines_behavior() {
        let src = "a\r\nb\r\n";
        let lines = highlight_file(src, Lang::Rust, Theme::Dark).expect("rust highlights");
        assert_eq!(lines.len(), src.lines().count());
        let flat: String = lines
            .iter()
            .map(|l| l.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(flat, "a\nb");
    }
    #[test]
    fn highlight_unknown_config_returns_none() {
        assert!(highlight_file("# hi\n", Lang::Markdown, Theme::Dark).is_none());
    }
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
}
