use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub fn viewer_hint(searching: bool) -> &'static str {
    if searching {
        "Enter ok · Esc cancel"
    } else {
        "j/k move · ^F/^B/^D/^U half · w/b/e word · 0^$ line · / n/N · g/G · ^W wrap · v/V select · Enter comment · C sidebar · q quit"
    }
}

/// Status mode tag for an active visual selection (`""` when none).
pub fn visual_tag(visual: Option<crate::select::Selection>) -> &'static str {
    match visual {
        Some(s) if s.kind == crate::select::SelectKind::Line => "--VISUAL LINE--",
        Some(_) => "--VISUAL--",
        None => "",
    }
}

/// Trailing status count (`[n comments]`), shown once comments exist.
pub fn comments_tag(n: usize) -> String {
    format!(" [{n} comments]")
}

/// Left status + right-aligned hint padded to `width` chars (char count,
/// not bytes). Hint keeps a subtle style; truncates on narrow widths.
pub fn status_line(width: usize, left: &str, right: &str) -> Line<'static> {
    let subtle = Style::default().fg(Color::DarkGray);
    let lw = left.chars().count();
    if width == 0 {
        return Line::from(vec![Span::raw(left.to_string())]);
    }
    if lw >= width {
        let t: String = left.chars().take(width).collect();
        return Line::from(vec![Span::raw(t)]);
    }
    let rw = right.chars().count();
    if lw + 1 + rw <= width {
        let mut l = left.to_string();
        l.push_str(&" ".repeat(width - lw - rw));
        return Line::from(vec![Span::raw(l), Span::styled(right.to_string(), subtle)]);
    }
    let keep = width.saturating_sub(lw + 1);
    let t: String = right.chars().take(keep).collect();
    let mut l = left.to_string();
    l.push(' ');
    Line::from(vec![Span::raw(l), Span::styled(t, subtle)])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hint_lists_compact_navigation_keys() {
        let h = viewer_hint(false);
        for k in [
            "j/k", "^F/^B", "^D/^U", "w/b/e", "0^$", "/ n/N", "g/G", "^W wrap", "q quit",
        ] {
            assert!(h.contains(k), "hint missing {k}: {h}");
        }
    }
    #[test]
    fn hint_searching_shows_confirm_cancel() {
        let h = viewer_hint(true);
        assert!(h.contains("Enter") && h.contains("Esc"), "unexpected: {h}");
    }
    #[test]
    fn status_line_pads_right_hint_to_width_with_subtle_style() {
        let line = status_line(20, "left", "right");
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(text.chars().count(), 20);
        assert!(text.starts_with("left"));
        assert!(text.ends_with("right"));
        let right_style = line.spans.last().unwrap().style;
        assert_eq!(right_style.fg, Some(Color::DarkGray));
    }
    #[test]
    fn status_line_truncates_hint_on_narrow_width() {
        let line = status_line(6, "left", "verylonghint");
        let text: String = line.spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(text.chars().count(), 6);
        assert!(text.starts_with("left"));
    }
    #[test]
    fn visual_tag_names_char_and_line_modes() {
        use crate::select::{SelectKind, Selection};
        let char_sel = Some(Selection { anchor: (0, 0), kind: SelectKind::Char });
        let line_sel = Some(Selection { anchor: (0, 0), kind: SelectKind::Line });
        assert_eq!(visual_tag(char_sel), "--VISUAL--");
        assert_eq!(visual_tag(line_sel), "--VISUAL LINE--");
        assert_eq!(visual_tag(None), "");
    }
    #[test]
    fn comments_tag_counts() {
        assert_eq!(comments_tag(0), " [0 comments]");
        assert_eq!(comments_tag(2), " [2 comments]");
    }
}
