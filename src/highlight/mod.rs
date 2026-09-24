pub mod langs;
pub mod theme;

pub use theme::{detect_theme, style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust, Python, JavaScript, TypeScript, Tsx, Html, Ruby, Elixir, Php, C, Toml, Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme { Dark, Light }

use std::path::Path;
use ratatui::text::Span;
use tree_sitter_highlight::{Highlighter, HighlightEvent};

/// Capture names recognized for styling. Order = style index in theme.rs.
/// Query captures not listed here produce no highlight event.
pub const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute", "boolean", "comment", "constant", "constant.builtin",
    "constructor", "function", "function.builtin", "function.macro",
    "keyword", "label", "number", "operator", "property",
    "punctuation", "string", "tag", "type", "type.builtin",
    "variable", "variable.builtin", "variable.parameter",
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
/// config (None) or empty text -> plain single-style lines. Never drops text.
pub fn highlight_file(text: &str, lang: Lang, theme: Theme) -> Vec<Vec<Span<'static>>> {
    let Some(config) = langs::configuration(lang) else {
        return text.lines().map(|l| vec![Span::raw(l.to_string())]).collect();
    };
    let mut hl = Highlighter::new();
    let Ok(events) = hl.highlight(config, text.as_bytes(), None, None, |_| None) else {
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
        let flat: String = lines.iter().map(|l| l.iter().map(|s| s.content.as_ref().to_string()).collect::<String>()).collect::<Vec<_>>().join("\n");
        assert_eq!(flat.trim_end_matches('\n'), src.trim_end_matches('\n'), "{lang:?} lost text");
    }
}
}
