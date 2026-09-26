// Task 1 provides pure helpers; callers land in a later task.
// Allow dead code until then so `cargo clippy -- -D warnings` stays clean.
#![allow(dead_code)]

pub const MAX_MATCHES: usize = 10_000;

pub fn find_matches(lines: &[String], query: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if query.is_empty() {
        return out;
    }
    let case_sensitive = query.chars().any(|c| c.is_uppercase());
    let q: Vec<char> = query.chars().collect();
    for (li, line) in lines.iter().enumerate() {
        let lchars: Vec<char> = line.chars().collect();
        if lchars.len() < q.len() {
            continue;
        }
        for start in 0..=(lchars.len() - q.len()) {
            let matched = if case_sensitive {
                lchars[start..start + q.len()] == q[..]
            } else {
                lchars[start..start + q.len()]
                    .iter()
                    .zip(q.iter())
                    .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
            };
            if matched {
                out.push((li, start));
                if out.len() >= MAX_MATCHES {
                    return out;
                }
            }
        }
    }
    out
}

pub fn scroll_for_match(match_row: usize, scroll: usize, vh: usize, margin: usize) -> usize {
    if vh == 0 {
        return scroll;
    }
    let m = margin.min((vh / 2).saturating_sub(1));
    let top = scroll.saturating_add(m);
    let bottom = scroll.saturating_add(vh).saturating_sub(m);
    let result = if match_row >= top && match_row < bottom {
        scroll
    } else if match_row < top {
        match_row.saturating_sub(m)
    } else {
        match_row
            .saturating_add(1)
            .saturating_add(m)
            .saturating_sub(vh)
    };
    crate::viewer::clamp_scroll(result, usize::MAX, vh)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literal_basic_and_overlap() {
        let lines = vec!["aaa".to_string()];
        assert_eq!(find_matches(&lines, "aa"), vec![(0, 0), (0, 1)]);
    }
    #[test]
    fn empty_query_is_empty() {
        let lines = vec!["hi".to_string()];
        assert!(find_matches(&lines, "").is_empty());
    }
    #[test]
    fn smart_case_insensitive_then_sensitive() {
        let lines = vec!["Hello hello".to_string()];
        assert_eq!(find_matches(&lines, "hello"), vec![(0, 0), (0, 6)]);
        assert_eq!(find_matches(&lines, "Hello"), vec![(0, 0)]);
    }
    #[test]
    fn unicode_char_cols() {
        let lines = vec!["héllo 🌍x".to_string()];
        assert_eq!(find_matches(&lines, "🌍"), vec![(0, 6)]);
    }
    #[test]
    fn scroll_middle_untouched() {
        assert_eq!(scroll_for_match(10, 5, 20, 2), 5);
    }
    #[test]
    fn scroll_near_bottom_edge() {
        assert_eq!(scroll_for_match(18, 0, 20, 2), 1);
    }
    #[test]
    fn scroll_outside_below() {
        assert_eq!(scroll_for_match(30, 0, 20, 2), 13);
    }
    #[test]
    fn scroll_outside_above() {
        assert_eq!(scroll_for_match(0, 10, 20, 2), 0);
    }
    #[test]
    fn scroll_tiny_viewport_saturates() {
        assert_eq!(scroll_for_match(2, 0, 3, 2), 0);
    }
}
