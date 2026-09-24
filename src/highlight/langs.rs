use super::{Lang, HIGHLIGHT_NAMES};
use std::sync::LazyLock;
use tree_sitter_highlight::HighlightConfiguration;

fn cfg(
    language: tree_sitter::Language,
    name: &str,
    highlight_query: &str,
    injection_query: &str,
    locals_query: &str,
) -> HighlightConfiguration {
    let mut c = HighlightConfiguration::new(
        language,
        name,
        highlight_query,
        injection_query,
        locals_query,
    )
    .expect("grammar queries load");
    c.configure(HIGHLIGHT_NAMES);
    c
}

#[cfg(feature = "lang-rust")]
static RUST: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_rust::LANGUAGE.into(),
        "rust",
        tree_sitter_rust::HIGHLIGHTS_QUERY,
        tree_sitter_rust::INJECTIONS_QUERY,
        "",
    )
});

#[cfg(feature = "lang-python")]
static PYTHON: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_python::LANGUAGE.into(),
        "python",
        tree_sitter_python::HIGHLIGHTS_QUERY,
        "",
        "",
    )
});

#[cfg(feature = "lang-javascript")]
static JAVASCRIPT: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_javascript::LANGUAGE.into(),
        "javascript",
        tree_sitter_javascript::HIGHLIGHT_QUERY,
        tree_sitter_javascript::INJECTIONS_QUERY,
        tree_sitter_javascript::LOCALS_QUERY,
    )
});

#[cfg(feature = "lang-typescript")]
static TYPESCRIPT: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "typescript",
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
        "",
        tree_sitter_typescript::LOCALS_QUERY,
    )
});

#[cfg(feature = "lang-typescript")]
static TSX: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_typescript::LANGUAGE_TSX.into(),
        "tsx",
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
        "",
        tree_sitter_typescript::LOCALS_QUERY,
    )
});

#[cfg(feature = "lang-html")]
static HTML: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_html::LANGUAGE.into(),
        "html",
        tree_sitter_html::HIGHLIGHTS_QUERY,
        tree_sitter_html::INJECTIONS_QUERY,
        "",
    )
});

#[cfg(feature = "lang-ruby")]
static RUBY: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_ruby::LANGUAGE.into(),
        "ruby",
        tree_sitter_ruby::HIGHLIGHTS_QUERY,
        "",
        tree_sitter_ruby::LOCALS_QUERY,
    )
});

#[cfg(feature = "lang-elixir")]
static ELIXIR: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_elixir::LANGUAGE.into(),
        "elixir",
        tree_sitter_elixir::HIGHLIGHTS_QUERY,
        tree_sitter_elixir::INJECTIONS_QUERY,
        "",
    )
});

#[cfg(feature = "lang-php")]
static PHP: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_php::LANGUAGE_PHP.into(),
        "php",
        tree_sitter_php::HIGHLIGHTS_QUERY,
        tree_sitter_php::INJECTIONS_QUERY,
        "",
    )
});

#[cfg(feature = "lang-c")]
static C: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_c::LANGUAGE.into(),
        "c",
        tree_sitter_c::HIGHLIGHT_QUERY,
        "",
        "",
    )
});

#[cfg(feature = "lang-toml")]
static TOML: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    cfg(
        tree_sitter_toml_ng::LANGUAGE.into(),
        "toml",
        tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
        "",
        "",
    )
});

// tree-sitter-markdown query layout differs; plain fallback (follow-up)

/// Borrow the configuration for a language, or None if its feature is off.
pub fn configuration(lang: Lang) -> Option<&'static HighlightConfiguration> {
    match lang {
        #[cfg(feature = "lang-rust")]
        Lang::Rust => Some(&RUST),
        #[cfg(feature = "lang-python")]
        Lang::Python => Some(&PYTHON),
        #[cfg(feature = "lang-javascript")]
        Lang::JavaScript => Some(&JAVASCRIPT),
        #[cfg(feature = "lang-typescript")]
        Lang::TypeScript => Some(&TYPESCRIPT),
        #[cfg(feature = "lang-typescript")]
        Lang::Tsx => Some(&TSX),
        #[cfg(feature = "lang-html")]
        Lang::Html => Some(&HTML),
        #[cfg(feature = "lang-ruby")]
        Lang::Ruby => Some(&RUBY),
        #[cfg(feature = "lang-elixir")]
        Lang::Elixir => Some(&ELIXIR),
        #[cfg(feature = "lang-php")]
        Lang::Php => Some(&PHP),
        #[cfg(feature = "lang-c")]
        Lang::C => Some(&C),
        #[cfg(feature = "lang-toml")]
        Lang::Toml => Some(&TOML),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::Lang;
    #[test]
    fn all_enabled_languages_load() {
        for lang in [
            Lang::Rust,
            Lang::Python,
            Lang::JavaScript,
            Lang::TypeScript,
            Lang::Tsx,
            Lang::Html,
            Lang::Ruby,
            Lang::Elixir,
            Lang::Php,
            Lang::C,
            Lang::Toml,
        ] {
            assert!(
                configuration(lang).is_some(),
                "{lang:?} has no configuration"
            );
        }
    }
    #[test]
    fn markdown_falls_back_to_plain() {
        // tree-sitter-markdown 0.7.1 ships no highlight queries; plain fallback (follow-up).
        assert!(configuration(Lang::Markdown).is_none());
    }
}
