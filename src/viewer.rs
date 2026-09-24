pub fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![line.to_string()];
    }
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return vec![String::new()];
    }
    chars.chunks(width).map(|c| c.iter().collect()).collect()
}
pub fn build_display_lines(lines: &[String], width: usize, wrap: bool) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let w = width.max(1);
    for (i, line) in lines.iter().enumerate() {
        if wrap {
            for chunk in wrap_line(line, w) {
                out.push((i, chunk));
            }
        } else {
            out.push((i, line.clone()));
        }
    }
    out
}
pub fn clamp_scroll(scroll: usize, total: usize, viewport: usize) -> usize {
    if total <= viewport || viewport == 0 {
        return 0;
    }
    scroll.min(total - viewport)
}
pub fn clamp_hscroll(h: usize, max_width: usize, viewport_w: usize) -> usize {
    if max_width <= viewport_w || viewport_w == 0 {
        return 0;
    }
    h.min(max_width - viewport_w)
}

/// Display-row index containing logical (cursor_line, cursor_col).
/// Wrap: rows of earlier lines (expanded) + chunk of this line holding the col.
pub fn display_row_for_cursor(
    lines: &[String],
    width: usize,
    wrap: bool,
    cursor_line: usize,
    cursor_col: usize,
) -> usize {
    if !wrap {
        return cursor_line.min(lines.len().saturating_sub(1));
    }
    let w = width.max(1);
    let mut row = 0;
    for (i, line) in lines.iter().enumerate() {
        let chunks = wrap_line(line, w).len().max(1);
        if i < cursor_line {
            row += chunks;
        } else if i == cursor_line {
            let nchars = line.chars().count();
            let col = cursor_col.min(nchars);
            row += (col / w).min(chunks - 1);
            break;
        } else {
            break;
        }
    }
    row
}

/// Visible text area inside a `Borders::ALL` block: subtract 2 rows/cols for
/// the border, then gutter + 1 space for line numbers. Saturates; minimum 1
/// col (callers need width >= 1), 0 rows allowed (renders nothing).
pub fn content_size(outer_w: usize, outer_h: usize, gutter: usize) -> (usize, usize) {
    let text_w = outer_w.saturating_sub(2).saturating_sub(gutter + 1).max(1);
    let vh = outer_h.saturating_sub(2);
    (text_w, vh)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn content_size_subtracts_borders_and_gutter() {
        // Outer 80x24 chunk with Borders::ALL (2 rows, 2 cols) and gutter 5:
        // text width = 80 - 2 - 5 - 1, height = 24 - 2.
        assert_eq!(content_size(80, 24, 5), (72, 22));
        // Saturates instead of underflowing on tiny areas.
        assert_eq!(content_size(3, 1, 5), (1, 0));
    }
    #[test]
    fn wrap_short_line_unchanged() {
        assert_eq!(wrap_line("hi", 10), vec!["hi".to_string()]);
    }
    #[test]
    fn wrap_long_line_splits_on_chars() {
        assert_eq!(
            wrap_line("abcdef", 2),
            vec!["ab".to_string(), "cd".to_string(), "ef".to_string()]
        );
    }
    #[test]
    fn wrap_zero_width_returns_whole() {
        assert_eq!(wrap_line("abc", 0), vec!["abc".to_string()]);
    }
    #[test]
    fn clamp_scroll_basic() {
        assert_eq!(clamp_scroll(0, 10, 5), 0);
        assert_eq!(clamp_scroll(100, 10, 5), 5);
        assert_eq!(clamp_scroll(3, 3, 5), 0);
        assert_eq!(clamp_scroll(0, 0, 5), 0);
    }
    #[test]
    fn build_display_unwrapped_keeps_logical_index() {
        let lines = vec!["a".to_string(), "b".to_string()];
        let d = build_display_lines(&lines, 10, false);
        assert_eq!(d, vec![(0, "a".to_string()), (1, "b".to_string())]);
    }
    #[test]
    fn build_display_wrapped_expands() {
        let lines = vec!["abcdef".to_string()];
        let d = build_display_lines(&lines, 2, true);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0], (0, "ab".to_string()));
    }
    #[test]
    fn cursor_display_row_unwrapped() {
        let lines = vec!["aaa".to_string(), "bbb".to_string()];
        assert_eq!(display_row_for_cursor(&lines, 10, false, 1, 0), 1);
    }
    #[test]
    fn cursor_display_row_wrapped() {
        // "abcdef" at width 2 -> 3 display rows; col 3 lives in 2nd chunk.
        let lines = vec!["abcdef".to_string(), "xy".to_string()];
        assert_eq!(display_row_for_cursor(&lines, 2, true, 0, 3), 1);
        assert_eq!(display_row_for_cursor(&lines, 2, true, 1, 0), 3);
    }
}
