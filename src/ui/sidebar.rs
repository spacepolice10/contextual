use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

/// Sidebar rows for the comments panel: per comment, a dim
/// `file: sL:sC → eL:eC` caption, the blueish snippet, then the note —
/// each wrapped (char-based, never truncated) to `width`. Shows the
/// most recent whole items that fit `height` (at least the newest,
/// cut to `height` when even it overflows).
pub fn sidebar_lines(
    app: &crate::app::App,
    height: usize,
    width: usize,
) -> Vec<Line<'static>> {
    if app.comments.is_empty() {
        return vec![Line::from(Span::raw(
            "No comments — v select, Enter comment",
        ))];
    }
    let w = width.max(1);
    let h = height.max(1);
    let dim = Style::default().fg(Color::DarkGray);
    let snip_style = Style::default().fg(Color::LightBlue);
    // Oldest → newest blocks; each block wraps its three parts.
    let blocks: Vec<Vec<Line<'static>>> = app
        .comments
        .iter()
        .map(|c| {
            let flat: String = c
                .snippet
                .replace('\r', "")
                .replace('\n', " ")
                .chars()
                .collect();
            let mut rows = Vec::new();
            for cap in wrap_text(
                &format!(
                    "{}: {}:{} → {}:{}",
                    c.file,
                    c.start.0 + 1,
                    c.start.1 + 1,
                    c.end.0 + 1,
                    c.end.1 + 1,
                ),
                w,
            ) {
                rows.push(Line::from(Span::styled(cap, dim)));
            }
            for s in wrap_text(&flat, w) {
                rows.push(Line::from(Span::styled(s, snip_style)));
            }
            for n in wrap_text(&c.note.replace('\r', ""), w) {
                rows.push(Line::from(Span::raw(n)));
            }
            rows
        })
        .collect();
    // Most-recent-first, whole items only.
    let mut picked: Vec<&Vec<Line<'static>>> = Vec::new();
    let mut used = 0;
    for b in blocks.iter().rev() {
        if used + b.len() <= h {
            picked.push(b);
            used += b.len();
        } else {
            break;
        }
    }
    if picked.is_empty() {
        // Even the newest overflows: show its head, never empty.
        return blocks
            .last()
            .map(|b| b.iter().take(h).cloned().collect())
            .unwrap_or_default();
    }
    picked.reverse();
    picked.into_iter().flatten().cloned().collect()
}

/// Char-based wrap of `text` into `width` columns: hard newlines split
/// first, then each part is cut into `width`-char chunks. Never drops
/// text, never panics on unicode; empty input yields one empty row.
pub(crate) fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let w = width.max(1);
    let mut out = Vec::new();
    for part in text.split('\n') {
        let chars: Vec<char> = part.chars().collect();
        if chars.is_empty() {
            out.push(String::new());
        } else {
            out.extend(chars.chunks(w).map(|c| c.iter().collect()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn viewer() -> crate::app::App {
        crate::app::App::load_file(std::path::Path::new("Cargo.toml"), false).unwrap()
    }
    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.to_string()).collect()
    }
    #[test]
    fn sidebar_lists_comment_rows() {
        let mut a = viewer();
        a.comments.push(crate::app::Comment {
            id: 0,
            file: "f.rs".to_string(),
            start: (0, 0),
            end: (0, 2),
            snippet: "hi".to_string(),
            note: "n".to_string(),
        });
        let rows = sidebar_lines(&a, 10, 40);
        assert_eq!(rows.len(), 3);
        let text: String = rows.iter().map(line_text).collect();
        assert!(text.contains("f.rs: 1:1 → 1:3"), "unexpected: {text}");
        assert!(text.contains("hi"), "unexpected: {text}");
        assert!(text.contains('n'), "unexpected: {text}");
        assert_eq!(rows[0].spans[0].style.fg, Some(Color::DarkGray));
        assert_eq!(rows[1].spans[0].style.fg, Some(Color::LightBlue));
    }
    #[test]
    fn sidebar_snippet_wraps_to_width() {
        let mut a = viewer();
        a.comments.push(crate::app::Comment {
            id: 0,
            file: "f.rs".to_string(),
            start: (0, 0),
            end: (0, 10),
            snippet: "abcdefghij".to_string(),
            note: "n".to_string(),
        });
        let rows = sidebar_lines(&a, 10, 4);
        let text: Vec<String> = rows.iter().map(line_text).collect();
        // Width 4 wraps everything: 4 caption rows + 3 snippet + 1 note.
        assert_eq!(
            text,
            vec!["f.rs", ": 1:", "1 → ", "1:11", "abcd", "efgh", "ij", "n"]
        );
    }
    #[test]
    fn sidebar_note_wraps_without_loss() {
        let mut a = viewer();
        a.comments.push(crate::app::Comment {
            id: 0,
            file: "f.rs".to_string(),
            start: (0, 0),
            end: (0, 2),
            snippet: "hi".to_string(),
            note: "abcdefghij".to_string(),
        });
        let rows = sidebar_lines(&a, 10, 4);
        // 4 caption rows + 1 snippet row + 3 note rows; every row fits width.
        assert_eq!(rows.len(), 8);
        assert!(rows.iter().all(|l| line_text(l).chars().count() <= 4));
        let note: String = rows[5..].iter().map(line_text).collect();
        assert_eq!(note, "abcdefghij");
    }
    #[test]
    fn sidebar_height_fits_most_recent_whole_items() {
        let mut a = viewer();
        for (id, file) in [(0, "f.rs"), (1, "g.rs")] {
            a.comments.push(crate::app::Comment {
                id,
                file: file.to_string(),
                start: (0, 0),
                end: (0, 2),
                snippet: "hi".to_string(),
                note: "n".to_string(),
            });
        }
        // Height 5 fits one 3-row item: the most recent wins, whole.
        let rows = sidebar_lines(&a, 5, 40);
        assert_eq!(rows.len(), 3);
        assert!(line_text(&rows[0]).contains("g.rs"), "unexpected");
    }
    #[test]
    fn sidebar_empty_shows_help() {
        let a = viewer();
        assert!(a.comments.is_empty());
        let rows = sidebar_lines(&a, 10, 40);
        let text: String = rows.iter().map(line_text).collect();
        assert!(text.contains("No comments"), "unexpected: {text}");
    }
}
