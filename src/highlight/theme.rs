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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;
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
