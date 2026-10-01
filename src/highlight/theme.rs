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
        (_, "comment") => match theme {
            Theme::Dark | Theme::Light => rgb(92, 99, 112).add_modifier(Modifier::ITALIC),
            Theme::Catppuccin => rgb(108, 112, 134).add_modifier(Modifier::ITALIC),
            Theme::Nord => rgb(76, 86, 106).add_modifier(Modifier::ITALIC),
            Theme::Dracula => rgb(98, 114, 164).add_modifier(Modifier::ITALIC),
            Theme::Gruvbox => rgb(146, 131, 116).add_modifier(Modifier::ITALIC),
            Theme::TokyoNight => rgb(86, 95, 137).add_modifier(Modifier::ITALIC),
            Theme::Monokai => rgb(117, 113, 94).add_modifier(Modifier::ITALIC),
            Theme::SolarizedDark => rgb(88, 110, 117).add_modifier(Modifier::ITALIC),
            Theme::SolarizedLight => rgb(147, 161, 161).add_modifier(Modifier::ITALIC),
            Theme::GithubDark => rgb(139, 148, 158).add_modifier(Modifier::ITALIC),
            Theme::RosePine => rgb(110, 106, 134).add_modifier(Modifier::ITALIC),
            Theme::AyuDark => rgb(98, 106, 115).add_modifier(Modifier::ITALIC),
            Theme::Everforest => rgb(133, 146, 137).add_modifier(Modifier::ITALIC),
            Theme::Synthwave => rgb(109, 109, 138).add_modifier(Modifier::ITALIC),
        },
        (_, "string") => match theme {
            Theme::Dark => rgb(152, 195, 121),
            Theme::Light => rgb(80, 130, 50),
            Theme::Catppuccin => rgb(166, 227, 161),
            Theme::Nord => rgb(163, 190, 140),
            Theme::Dracula => rgb(241, 250, 140),
            Theme::Gruvbox => rgb(184, 187, 38),
            Theme::TokyoNight => rgb(158, 206, 106),
            Theme::Monokai => rgb(230, 219, 116),
            Theme::SolarizedDark | Theme::SolarizedLight => rgb(42, 161, 152),
            Theme::GithubDark => rgb(165, 214, 255),
            Theme::RosePine => rgb(235, 188, 186),
            Theme::AyuDark => rgb(194, 217, 76),
            Theme::Everforest => rgb(167, 192, 128),
            Theme::Synthwave => rgb(253, 238, 93),
        },
        (_, "keyword") => match theme {
            Theme::Dark => rgb(198, 120, 221),
            Theme::Light => rgb(160, 60, 180),
            Theme::Catppuccin => rgb(203, 166, 247),
            Theme::Nord => rgb(180, 142, 173),
            Theme::Dracula => rgb(255, 121, 198),
            Theme::Gruvbox => rgb(251, 73, 52),
            Theme::TokyoNight => rgb(187, 154, 247),
            Theme::Monokai => rgb(249, 38, 114),
            Theme::SolarizedDark | Theme::SolarizedLight => rgb(133, 153, 0),
            Theme::GithubDark => rgb(255, 123, 114),
            Theme::RosePine => rgb(235, 111, 147),
            Theme::AyuDark => rgb(255, 143, 64),
            Theme::Everforest => rgb(230, 126, 128),
            Theme::Synthwave => rgb(255, 126, 219),
        },
        (_, "type") | (_, "type.builtin") => match theme {
            Theme::Dark => rgb(229, 192, 123),
            Theme::Light => rgb(170, 120, 40),
            Theme::Catppuccin => rgb(249, 226, 175),
            Theme::Nord => rgb(235, 203, 139),
            Theme::Dracula => rgb(189, 147, 249),
            Theme::Gruvbox => rgb(250, 189, 47),
            Theme::TokyoNight => rgb(224, 175, 104),
            Theme::Monokai => rgb(102, 217, 239),
            Theme::SolarizedDark | Theme::SolarizedLight => rgb(181, 137, 0),
            Theme::GithubDark => rgb(255, 166, 87),
            Theme::RosePine => rgb(246, 193, 119),
            Theme::AyuDark => rgb(115, 208, 255),
            Theme::Everforest => rgb(211, 181, 138),
            Theme::Synthwave => rgb(54, 249, 246),
        },
        (_, "function") | (_, "function.builtin") | (_, "function.macro") => match theme {
            Theme::Dark => rgb(97, 175, 239),
            Theme::Light => rgb(30, 100, 200),
            Theme::Catppuccin => rgb(137, 180, 250),
            Theme::Nord => rgb(136, 192, 208),
            Theme::Dracula => rgb(80, 250, 123),
            Theme::Gruvbox => rgb(131, 165, 152),
            Theme::TokyoNight => rgb(122, 162, 247),
            Theme::Monokai => rgb(166, 226, 46),
            Theme::SolarizedDark | Theme::SolarizedLight => rgb(38, 139, 210),
            Theme::GithubDark => rgb(210, 168, 255),
            Theme::RosePine => rgb(196, 167, 231),
            Theme::AyuDark => rgb(255, 213, 128),
            Theme::Everforest => rgb(219, 188, 127),
            Theme::Synthwave => rgb(114, 241, 184),
        },
        (_, "number") | (_, "boolean") => match theme {
            Theme::Dark => rgb(209, 154, 102),
            Theme::Light => rgb(180, 110, 40),
            Theme::Catppuccin => rgb(250, 179, 135),
            Theme::Nord => rgb(208, 135, 112),
            Theme::Dracula => rgb(255, 184, 108),
            Theme::Gruvbox => rgb(254, 128, 25),
            Theme::TokyoNight => rgb(255, 158, 100),
            Theme::Monokai => rgb(174, 129, 255),
            Theme::SolarizedDark | Theme::SolarizedLight => rgb(211, 54, 130),
            Theme::GithubDark => rgb(121, 192, 255),
            Theme::RosePine => rgb(156, 207, 216),
            Theme::AyuDark => rgb(210, 166, 255),
            Theme::Everforest => rgb(214, 153, 182),
            Theme::Synthwave => rgb(248, 143, 37),
        },
        (_, "constant") | (_, "constant.builtin") => match theme {
            Theme::Dark => rgb(86, 182, 194),
            Theme::Light => rgb(30, 140, 150),
            Theme::Catppuccin => rgb(148, 226, 213),
            Theme::Nord => rgb(143, 188, 187),
            Theme::Dracula => rgb(139, 233, 253),
            Theme::Gruvbox => rgb(142, 192, 124),
            Theme::TokyoNight => rgb(125, 207, 255),
            Theme::Monokai => rgb(174, 129, 255),
            Theme::SolarizedDark | Theme::SolarizedLight => rgb(42, 161, 152),
            Theme::GithubDark => rgb(121, 192, 255),
            Theme::RosePine => rgb(156, 207, 216),
            Theme::AyuDark => rgb(95, 216, 184),
            Theme::Everforest => rgb(131, 192, 146),
            Theme::Synthwave => rgb(54, 249, 246),
        },
        (_, "variable") | (_, "variable.builtin") | (_, "variable.parameter") => match theme {
            Theme::Dark => rgb(224, 108, 117),
            Theme::Light => rgb(190, 60, 70),
            Theme::Catppuccin => rgb(243, 139, 168),
            Theme::Nord => rgb(191, 97, 106),
            Theme::Dracula => rgb(255, 85, 85),
            Theme::Gruvbox => rgb(235, 219, 178),
            Theme::TokyoNight => rgb(192, 202, 245),
            Theme::Monokai => rgb(248, 248, 242),
            Theme::SolarizedDark => rgb(131, 148, 150),
            Theme::SolarizedLight => rgb(101, 123, 131),
            Theme::GithubDark => rgb(201, 209, 217),
            Theme::RosePine => rgb(224, 222, 244),
            Theme::AyuDark => rgb(230, 225, 207),
            Theme::Everforest => rgb(211, 198, 170),
            Theme::Synthwave => rgb(248, 248, 242),
        },
        (_, "property") | (_, "tag") | (_, "attribute") => match theme {
            Theme::Dark => rgb(229, 192, 123),
            Theme::Light => rgb(170, 120, 40),
            Theme::Catppuccin => rgb(249, 226, 175),
            Theme::Nord => rgb(235, 203, 139),
            Theme::Dracula => rgb(189, 147, 249),
            Theme::Gruvbox => rgb(250, 189, 47),
            Theme::TokyoNight => rgb(224, 175, 104),
            Theme::Monokai => rgb(102, 217, 239),
            Theme::SolarizedDark | Theme::SolarizedLight => rgb(181, 137, 0),
            Theme::GithubDark => rgb(255, 166, 87),
            Theme::RosePine => rgb(246, 193, 119),
            Theme::AyuDark => rgb(115, 208, 255),
            Theme::Everforest => rgb(211, 181, 138),
            Theme::Synthwave => rgb(253, 238, 93),
        },
        (_, "operator") | (_, "punctuation") => match theme {
            Theme::Dark => rgb(171, 178, 191),
            Theme::Light => rgb(90, 90, 90),
            Theme::Catppuccin => rgb(205, 214, 244),
            Theme::Nord => rgb(216, 222, 233),
            Theme::Dracula => rgb(248, 248, 242),
            Theme::Gruvbox => rgb(168, 153, 132),
            Theme::TokyoNight => rgb(137, 221, 255),
            Theme::Monokai => rgb(248, 248, 242),
            Theme::SolarizedDark => rgb(131, 148, 150),
            Theme::SolarizedLight => rgb(101, 123, 131),
            Theme::GithubDark => rgb(201, 209, 217),
            Theme::RosePine => rgb(144, 140, 170),
            Theme::AyuDark => rgb(230, 225, 207),
            Theme::Everforest => rgb(157, 169, 160),
            Theme::Synthwave => rgb(184, 147, 206),
        },
        (_, "constructor") | (_, "label") => match theme {
            Theme::Dark => rgb(97, 175, 239),
            Theme::Light => rgb(30, 100, 200),
            Theme::Catppuccin => rgb(137, 180, 250),
            Theme::Nord => rgb(136, 192, 208),
            Theme::Dracula => rgb(80, 250, 123),
            Theme::Gruvbox => rgb(131, 165, 152),
            Theme::TokyoNight => rgb(122, 162, 247),
            Theme::Monokai => rgb(166, 226, 46),
            Theme::SolarizedDark | Theme::SolarizedLight => rgb(38, 139, 210),
            Theme::GithubDark => rgb(210, 168, 255),
            Theme::RosePine => rgb(196, 167, 231),
            Theme::AyuDark => rgb(255, 213, 128),
            Theme::Everforest => rgb(219, 188, 127),
            Theme::Synthwave => rgb(114, 241, 184),
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
            let idx = super::super::HIGHLIGHT_NAMES
                .iter()
                .position(|n| *n == name)
                .unwrap();
            assert_ne!(style(Theme::Dark, idx), Style::default(), "dark {name}");
            assert_ne!(style(Theme::Light, idx), Style::default(), "light {name}");
        }
    }
    #[test]
    fn all_builtin_themes_style_known_captures() {
        // Every builtin theme must style the core captures (never default).
        for theme in Theme::all() {
            for name in ["keyword", "string", "comment", "type", "function", "number"] {
                let idx = super::super::HIGHLIGHT_NAMES
                    .iter()
                    .position(|n| *n == name)
                    .unwrap();
                assert_ne!(style(*theme, idx), Style::default(), "{theme:?} {name}");
            }
        }
    }
    #[test]
    fn builtin_theme_list_has_popular_palettes() {
        let names: Vec<&str> = Theme::all().iter().map(|t| t.name()).collect();
        for want in ["catppuccin", "nord", "dracula"] {
            assert!(names.contains(&want), "missing {want} in {names:?}");
        }
    }
    #[test]
    fn new_interesting_themes_present() {
        let names: Vec<&str> = Theme::all().iter().map(|t| t.name()).collect();
        for want in [
            "monokai",
            "solarized-dark",
            "solarized-light",
            "github-dark",
            "rose-pine",
            "ayu-dark",
            "everforest",
            "synthwave",
        ] {
            assert!(names.contains(&want), "missing {want} in {names:?}");
        }
    }
    #[test]
    fn colorfgbg_dark_background() {
        assert_eq!(theme_from_colorfgbg(Some("15;0")), Theme::Dark);
        assert_eq!(theme_from_colorfgbg(Some("0;15")), Theme::Light);
        assert_eq!(theme_from_colorfgbg(None), Theme::Dark);
    }
}
