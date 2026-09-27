use ratatui::{
    style::{Color, Style},
    text::Span,
};

/// Repaint exact-match ranges with vim-like Search style (yellow bg).
/// Ranges are chunk-relative char offsets into the concatenated spans.
pub fn paint_search_ranges(
    spans: Vec<Span<'static>>,
    ranges: &[(usize, usize)],
) -> Vec<Span<'static>> {
    paint_ranges(spans, ranges, Style::default().bg(Color::Yellow).fg(Color::Black))
}

/// Repaint selection ranges with vim-like Visual style (blue bg).
/// Same chunk-relative contract as [`paint_search_ranges`].
pub fn paint_selection_ranges(
    spans: Vec<Span<'static>>,
    ranges: &[(usize, usize)],
) -> Vec<Span<'static>> {
    paint_ranges(spans, ranges, Style::default().bg(Color::Blue).fg(Color::White))
}

/// Shared cell repaint for chunk-relative ranges: explode spans to styled
/// chars, overwrite the style in each range, re-merge adjacent equals.
pub fn paint_ranges(
    spans: Vec<Span<'static>>,
    ranges: &[(usize, usize)],
    style: Style,
) -> Vec<Span<'static>> {
    let mut cells: Vec<(char, Style)> = Vec::new();
    for s in &spans {
        for c in s.content.chars() {
            cells.push((c, s.style));
        }
    }
    for &(a, b) in ranges {
        for i in a.min(cells.len())..b.min(cells.len()) {
            cells[i].1 = style;
        }
    }
    let mut out: Vec<Span<'static>> = Vec::new();
    for (c, st) in cells {
        let t = c.to_string();
        match out.last_mut() {
            Some(last) if last.style == st => last.content.to_mut().push_str(&t),
            _ => out.push(Span::styled(t, st)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_paint_uses_blue_bg() {
        let spans = vec![Span::raw("hello".to_string())];
        let out = paint_selection_ranges(spans, &[(1, 4)]);
        let text: String = out.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(text, "hello");
        assert!(
            out.iter().any(|s| s.style.bg == Some(Color::Blue)),
            "no blue cell painted"
        );
        assert_eq!(out.iter().filter(|s| s.style.bg == Some(Color::Blue)).count(), 1);
    }
}
