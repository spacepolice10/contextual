pub mod langs;
pub mod theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust, Python, JavaScript, TypeScript, Tsx, Html, Ruby, Elixir, Php, C, Toml, Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme { Dark, Light }

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
