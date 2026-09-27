use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

/// Sidebar rows for the comments panel: per comment, three rows —
/// dim `file: sL:sC → eL:eC` caption (1-based), blueish truncated
/// snippet (flattened to one row, cut to `width` + `…`), then the full
/// note. Shows the last `height / 3` comments (at least one, no scroll).
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
    const ROWS_PER_ITEM: usize = 3;
    let w = width.max(1);
    let n = (height / ROWS_PER_ITEM).max(1).min(app.comments.len());
    let dim = Style::default().fg(Color::DarkGray);
    let snip_style = Style::default().fg(Color::LightBlue);
    app.comments[app.comments.len() - n..]
        .iter()
        .flat_map(|c| {
            let flat: String = c
                .snippet
                .replace('\r', "")
                .replace('\n', " ")
                .chars()
                .collect();
            let snip = if flat.chars().count() > w {
                format!("{}…", flat.chars().take(w).collect::<String>())
            } else {
                flat
            };
            vec![
                Line::from(Span::styled(
                    format!(
                        "{}: {}:{} → {}:{}",
                        c.file,
                        c.start.0 + 1,
                        c.start.1 + 1,
                        c.end.0 + 1,
                        c.end.1 + 1,
                    ),
                    dim,
                )),
                Line::from(Span::styled(snip, snip_style)),
                Line::from(Span::raw(c.note.clone())),
            ]
        })
        .collect()
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
    fn sidebar_snippet_truncates_to_width() {
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
        assert_eq!(rows.len(), 3);
        let snip: String = rows[1].spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(snip, "abcd…");
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
