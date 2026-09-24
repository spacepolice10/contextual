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

#[cfg(test)]
mod tests {
    use super::*;
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
}
