use crate::ui::paint::{paint_search_ranges, paint_selection_ranges};
use crate::ui::sidebar::sidebar_lines;
use crate::ui::status::{comments_tag, status_line, viewer_hint, visual_tag};
use crate::{highlight, viewer};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Gutter number (vim `relativenumber` style): the cursor's own line
/// shows its absolute 1-based number, every other line shows the
/// distance from the cursor.
pub fn gutter_number(lidx: usize, cursor_line: usize) -> usize {
    if lidx == cursor_line {
        lidx + 1
    } else {
        lidx.abs_diff(cursor_line)
    }
}

/// Display row where the inline comment editor starts: right after the
/// last display row of `end_line`. Falls back to the very end when the
/// line is missing (never panics, never hides the editor).
pub fn editor_insert_row(display: &[(usize, String)], end_line: usize) -> usize {
    display
        .iter()
        .rposition(|(l, _)| *l == end_line)
        .map(|i| i + 1)
        .unwrap_or(display.len())
}

/// Inline comment editor rows: the single-line draft wrapped
/// (char-based) to `width`, gutter-aligned, with the block cursor cell
/// at the text end. A full last row pushes the cursor onto its own row
/// so it is never truncated. Text is never dropped.
pub fn comment_editor_rows(
    draft: &str,
    width: usize,
    gutter: usize,
) -> Vec<Line<'static>> {
    let w = width.max(1);
    let pad = " ".repeat(gutter);
    let cursor = Style::default().bg(Color::DarkGray).fg(Color::White);
    let chunks = crate::ui::sidebar::wrap_text(draft, w);
    let mut rows: Vec<Line<'static>> = chunks
        .iter()
        .map(|c| Line::from(Span::raw(format!("{pad}{c}"))))
        .collect();
    let full = chunks
        .last()
        .map(|c| c.chars().count() >= w)
        .unwrap_or(false);
    if full {
        rows.push(Line::from(vec![
            Span::raw(pad),
            Span::styled("▌", cursor),
        ]));
    } else if let Some(last) = rows.last_mut() {
        last.spans.push(Span::styled("▌", cursor));
    }
    rows
}

/// Viewer screen: text panel + optional comments sidebar + 2-row status bar.
pub fn render_viewer(f: &mut Frame, app: &mut crate::app::App, area: Rect) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(2)])
                .split(area);
            // Sidebar takes a fixed 50-col strip when toggled (`[text | 50]`).
            let (text_area, side_area) = if app.show_sidebar {
                let h = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(1), Constraint::Length(50)])
                    .split(chunks[0]);
                (h[0], Some(h[1]))
            } else {
                (chunks[0], None)
            };
            let gutter = app.lines.len().to_string().len().max(4) + 1;
            let (text_w, vh) =
                viewer::content_size(text_area.width as usize, text_area.height as usize, gutter);
            let display = viewer::build_display_lines(&app.lines, text_w.max(1), app.wrap);
            let total = display.len();
            app.viewport_h = vh;
            app.scroll = viewer::clamp_scroll(app.scroll, total, vh);
            let max_width = app
                .lines
                .iter()
                .map(|l| l.chars().count())
                .max()
                .unwrap_or(0);
            app.h_scroll = viewer::clamp_hscroll(app.h_scroll, max_width, text_w.max(1));
            // Keep cursor visible: vertical via display row, horizontal when unwrapped.
            let cursor_row = viewer::display_row_for_cursor(
                &app.lines,
                text_w.max(1),
                app.wrap,
                app.cursor_line,
                app.cursor_col,
            );
            app.ensure_cursor_visible(cursor_row, vh);
            app.scroll = viewer::clamp_scroll(app.scroll, total, vh);
            if !app.search_matches.is_empty() {
                let (ml, mc) = app.search_matches[app.search_idx.min(app.search_matches.len() - 1)];
                let mrow =
                    viewer::display_row_for_cursor(&app.lines, text_w.max(1), app.wrap, ml, mc);
                let m = if vh < 2 * 2 + 1 { 0 } else { 2 };
                let in_middle = mrow >= app.scroll + m && mrow < app.scroll + vh.saturating_sub(m);
                if !in_middle {
                    app.scroll = crate::search::scroll_for_match(mrow, app.scroll, vh, 2);
                    app.scroll = viewer::clamp_scroll(app.scroll, total, vh);
                }
            }
            // Match columns grouped by logical line for exact-substring painting.
            let mut line_matches: std::collections::HashMap<usize, Vec<usize>> =
                std::collections::HashMap::new();
            if !app.search_query.is_empty() {
                for &(ml, mc) in &app.search_matches {
                    line_matches.entry(ml).or_default().push(mc);
                }
            }
            let search_qlen = app.search_query.chars().count();
            if !app.wrap {
                let tw = text_w.max(1);
                if app.cursor_col < app.h_scroll {
                    app.h_scroll = app.cursor_col;
                } else if app.cursor_col >= app.h_scroll + tw {
                    app.h_scroll = app.cursor_col + 1 - tw;
                }
                app.h_scroll = viewer::clamp_hscroll(app.h_scroll, max_width, tw);
            }
            // Inline comment editor: virtual rows spliced into the text
            // below the selection end line. Scroll first so the editor
            // fits the viewport (tail wins when it overflows it).
            let editor = app.commenting.as_ref().map(|p| {
                let rows = comment_editor_rows(&p.draft, text_w.max(1), gutter);
                let at = editor_insert_row(&display, p.end.0);
                (rows, at)
            });
            if let Some((rows, at)) = &editor {
                let e = rows.len().max(1).min(vh.max(1));
                app.scroll = at.saturating_sub(vh.saturating_sub(e));
                app.scroll = viewer::clamp_scroll(app.scroll, total, vh);
            }
            let end = (app.scroll + vh).min(total);
            let cursor_style = Style::default().bg(Color::DarkGray).fg(Color::White);
            let mut text = Vec::new();
            // Chunk ordinal within the current logical line (wrap mode):
            // display rows for one line are consecutive, so count repeats.
            let mut prev_lidx: Option<usize> = None;
            let mut chunk_k: usize = 0;
            for (drow, (lidx, content)) in display[app.scroll..end].iter().enumerate() {
                let abs_row = app.scroll + drow;
                let _ = abs_row;
                match prev_lidx {
                    Some(p) if p == *lidx => chunk_k += 1,
                    _ => chunk_k = 0,
                }
                prev_lidx = Some(*lidx);
                let w = text_w.max(1);
                // Styled source for this logical line; .get() falls back to
                // plain when highlight lines diverge from text lines.
                let src = app.highlighted.as_ref().and_then(|h| h.get(*lidx));
                let shown = if app.wrap {
                    content.clone()
                } else {
                    content.chars().skip(app.h_scroll).take(w).collect()
                };
                // Visible window as styled spans. Plain path stays
                // byte-identical to the old `shown` string pipeline.
                let mut windowed: Vec<Span> = match src {
                    Some(spans) if app.wrap => highlight::slice_spans(spans, chunk_k * w, w),
                    Some(spans) => highlight::slice_spans(spans, app.h_scroll, w),
                    None => vec![Span::raw(shown.clone())],
                };
                let hl_row = src.is_some();
                // Paint exact match substrings yellow (vim-like Search). Runs
                // before the cursor split so the cursor cell still wins on the
                // current match's first cell.
                if search_qlen > 0 {
                    if let Some(cols) = line_matches.get(lidx) {
                        let coff = if app.wrap { chunk_k * w } else { app.h_scroll };
                        let nchars: usize =
                            windowed.iter().map(|s| s.content.chars().count()).sum();
                        let ranges =
                            crate::search::chunk_match_ranges(cols, search_qlen, coff, nchars);
                        if !ranges.is_empty() {
                            windowed = paint_search_ranges(windowed, &ranges);
                        }
                    }
                }
                // Paint the visual selection blue (vim-like Visual). Runs
                // after search paint so selection wins there, but before
                // the cursor split below so the cursor cell still wins.
                if let Some(sel) = app.visual {
                    let cursor = (app.cursor_line, app.cursor_col);
                    let (mut s0, mut e0) =
                        crate::select::normalize(sel.anchor, cursor, sel.kind);
                    if sel.kind == crate::select::SelectKind::Line {
                        s0 = (s0.0, 0);
                        e0 = (
                            e0.0,
                            app.lines
                                .get(e0.0)
                                .map(|l| l.chars().count())
                                .unwrap_or(0),
                        );
                    }
                    let coff = if app.wrap { chunk_k * w } else { app.h_scroll };
                    let nchars: usize =
                        windowed.iter().map(|s| s.content.chars().count()).sum();
                    let ranges =
                        crate::select::selection_chunks(s0, e0, *lidx, coff, nchars);
                    if !ranges.is_empty() {
                        windowed = paint_selection_ranges(windowed, &ranges);
                    }
                }
                // Highlight the cursor cell on the cursor's display row.
                let is_cursor_row = abs_row == cursor_row;
                let body_span = if !is_cursor_row {
                    windowed
                } else {
                    // Column offset inside this visible chunk.
                    let rel_col = if app.wrap {
                        app.cursor_col % w.max(1)
                    } else {
                        app.cursor_col.saturating_sub(app.h_scroll)
                    };
                    let nchars: usize = if hl_row {
                        windowed.iter().map(|s| s.content.chars().count()).sum()
                    } else {
                        shown.chars().count()
                    };
                    if rel_col >= nchars {
                        // Past end (or empty line): mark a space cell.
                        let mut v = if hl_row {
                            windowed
                        } else {
                            vec![Span::raw(shown)]
                        };
                        v.push(Span::styled(" ", cursor_style));
                        v
                    } else if hl_row {
                        let mut before = highlight::slice_spans(&windowed, 0, rel_col);
                        let mut cur = highlight::slice_spans(&windowed, rel_col, 1);
                        for s in &mut cur {
                            s.style = s.style.bg(Color::DarkGray);
                            if s.style.fg.is_none() {
                                s.style = s.style.fg(Color::White);
                            }
                        }
                        let after =
                            highlight::slice_spans(&windowed, rel_col + 1, nchars - rel_col - 1);
                        before.extend(cur);
                        before.extend(after);
                        before
                    } else {
                        // Plain path: split the (possibly search/selection
                        // painted) `windowed` spans so paint survives and
                        // only the cursor cell is restyled.
                        let mut cells: Vec<(char, Style)> = Vec::new();
                        for sp in &windowed {
                            for c in sp.content.chars() {
                                cells.push((c, sp.style));
                            }
                        }
                        let mut v: Vec<Span> = Vec::new();
                        for (i, (c, st)) in cells.into_iter().enumerate() {
                            let st = if i == rel_col { cursor_style } else { st };
                            let t = c.to_string();
                            match v.last_mut() {
                                Some(last) if last.style == st => {
                                    last.content.to_mut().push_str(&t);
                                }
                                _ => v.push(Span::styled(t, st)),
                            }
                        }
                        if rel_col >= nchars {
                            v.push(Span::styled(" ", cursor_style));
                        }
                        v
                    }
                };
                let mut spans = vec![Span::styled(
                    format!(
                        "{:>width$} ",
                        gutter_number(*lidx, app.cursor_line),
                        width = gutter - 1
                    ),
                    Style::default().fg(Color::DarkGray),
                )];
                spans.extend(body_span);
                text.push(Line::from(spans));
            }
            // Splice the inline editor below the selection end line; the
            // tail wins when it overflows the viewport (cursor stays seen).
            if let Some((rows, at)) = editor {
                let pos = at.saturating_sub(app.scroll).min(text.len());
                let room = vh.saturating_sub(pos);
                let take = room.min(rows.len());
                let skip = rows.len() - take;
                text.splice(pos..pos, rows.into_iter().skip(skip).take(take));
                text.truncate(vh);
            }
            let body = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(format!(" {} ", app.filename)),
            );
            f.render_widget(body, text_area);
            if let Some(side) = side_area {
                let inner_h = side.height.saturating_sub(2) as usize;
                let inner_w = side.width.saturating_sub(2) as usize;
                let side_rows = sidebar_lines(app, inner_h, inner_w);
                let panel = Paragraph::new(side_rows).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" comments (C) "),
                );
                f.render_widget(panel, side);
            }
            let mut s = format!(
                " {}/{}  Ln {},Col {}  wrap:{} ",
                app.scroll + 1,
                total.max(1),
                app.cursor_line + 1,
                app.cursor_col + 1,
                if app.wrap { "ON" } else { "OFF" }
            );
            if app.searching {
                if app.search_matches.is_empty() {
                    s = format!("/{}  [no matches]", app.search_query);
                } else {
                    let idx = app.search_idx.min(app.search_matches.len() - 1);
                    s = format!(
                        "/{}  {}/{}",
                        app.search_query,
                        idx + 1,
                        app.search_matches.len()
                    );
                }
            } else if !app.search_query.is_empty() && !app.search_matches.is_empty() {
                let idx = app.search_idx.min(app.search_matches.len() - 1);
                s.push_str(&format!(
                    "  /{} {}/{}",
                    app.search_query,
                    idx + 1,
                    app.search_matches.len()
                ));
            }
            if let Some(n) = &app.status_note {
                s.push_str(n);
            }
            if app.lang.is_some() && app.highlighted.is_none() {
                s.push_str("[no highlight]");
            }
            let tag = visual_tag(app.visual);
            if !tag.is_empty() {
                s.push(' ');
                s.push_str(tag);
            }
            if !app.comments.is_empty() {
                s.push_str(&comments_tag(app.comments.len()));
            }
            let bar_w = chunks[1].width as usize;
            // While typing a note the draft lives inline below the
            // selection; row 1 stays the normal status line and row 2
            // keeps the confirm hint.
            let hint = if app.commenting.is_some() {
                Line::from(Span::styled(
                    "Enter save · Esc cancel",
                    Style::default().fg(Color::DarkGray),
                ))
            } else {
                Line::from(Span::raw(""))
            };
            let rows = vec![status_line(bar_w, &s, viewer_hint(app.searching)), hint];
            let bar = Paragraph::new(rows);
            f.render_widget(bar, chunks[1]);
}
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::style::Color;
    fn viewer() -> crate::app::App {
        crate::app::App::load_file(std::path::Path::new("Cargo.toml"), false).unwrap()
    }
    #[test]
    fn gutter_number_relative_with_absolute_cursor_row() {
        // Cursor on line 5 (idx 4): own row absolute, others distance.
        assert_eq!(gutter_number(4, 4), 5);
        assert_eq!(gutter_number(0, 4), 4);
        assert_eq!(gutter_number(3, 4), 1);
        assert_eq!(gutter_number(5, 4), 1);
        assert_eq!(gutter_number(9, 4), 5);
        // Cursor on first line.
        assert_eq!(gutter_number(0, 0), 1);
        assert_eq!(gutter_number(2, 0), 2);
    }
    #[test]
    fn comment_editor_rows_wrap_and_mark_cursor() {
        // Width 4, gutter 2: draft wraps into aligned rows, ▌ ends the text.
        let rows = comment_editor_rows("abcdef", 4, 2);
        let text: Vec<String> = rows
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect())
            .collect();
        assert_eq!(text, vec!["  abcd".to_string(), "  ef▌".to_string()]);
        // Full last row pushes the cursor onto its own row (never truncated).
        let rows = comment_editor_rows("abcdefgh", 4, 2);
        let text: Vec<String> = rows
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect())
            .collect();
        assert_eq!(text, vec!["  abcd".to_string(), "  efgh".to_string(), "  ▌".to_string()]);
        // Empty draft is still one visible cursor row; text never lost.
        let rows = comment_editor_rows("", 4, 2);
        let flat: String = rows
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(flat.contains('▌'));
        let long = "x".repeat(100);
        let rows = comment_editor_rows(&long, 10, 0);
        let flat: String = rows
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect();
        assert_eq!(flat.chars().filter(|&c| c == 'x').count(), 100);
    }
    #[test]
    fn editor_insert_row_follows_selection_end() {
        let display = vec![
            (0, "a".to_string()),
            (1, "b".to_string()),
            (1, "c".to_string()),
            (2, "d".to_string()),
        ];
        assert_eq!(editor_insert_row(&display, 1), 3);
        assert_eq!(editor_insert_row(&display, 0), 1);
        assert_eq!(editor_insert_row(&display, 9), display.len());
    }
    #[test]
    fn comment_editor_renders_inline_below_selection() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut a = viewer();
        a.lines = vec!["aa".to_string(), "bb".to_string(), "cc".to_string()];
        a.lang = None;
        a.highlighted = None;
        a.cursor_line = 0;
        a.cursor_col = 1;
        a.visual = Some(crate::select::Selection {
            anchor: (0, 0),
            kind: crate::select::SelectKind::Char,
        });
        // Select (0,0)-(0,1), open the draft with a distinctive note.
        crate::input::handle(&mut a, KeyCode::Char('e'), KeyModifiers::NONE).unwrap();
        crate::input::handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(a.commenting.is_some());
        for c in "zz-note".chars() {
            crate::input::handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap();
        }
        let backend = TestBackend::new(40, 12);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| crate::render(f, &mut a)).unwrap();
        let buf = term.backend().buffer().clone();
        let text: String = buf.content().iter().map(|c| c.symbol().to_string()).collect();
        // Draft lives in the text panel now, not as a status-bar prompt.
        assert!(text.contains("zz-note"), "no inline editor text");
        assert!(!text.contains("Comment on"), "prompt still in status bar");
        // Status row 1 is the normal status line again.
        assert!(text.contains("Ln 2,Col 2"), "no normal status line");
        assert!(text.contains("Enter save"), "no save hint");
    }
    #[test]
    fn render_smoke_visual_sidebar_commenting() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut a = viewer();
        a.lines = vec!["hello world".to_string(), "second line".to_string()];
        a.lang = None;
        a.highlighted = None;
        crate::input::handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        crate::input::handle(&mut a, KeyCode::Char('e'), KeyModifiers::NONE).unwrap();
        crate::input::handle(&mut a, KeyCode::Char('j'), KeyModifiers::NONE).unwrap();
        assert!(a.visual.is_some());
        a.show_sidebar = true;
        a.comments.push(crate::app::Comment {
            id: 0,
            file: "f.rs".to_string(),
            start: (0, 0),
            end: (0, 4),
            snippet: "hello".to_string(),
            note: "n".to_string(),
        });
        a.commenting = Some(crate::app::PendingComment {
            snippet: "hello".to_string(),
            start: (0, 0),
            end: (0, 5),
            draft: "zz-draft-ok".to_string(),
        });
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| crate::render(f, &mut a)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(text.contains("comments (C)"), "no sidebar title");
        // Draft renders inline below the selection now: the note text is
        // in the text panel, the status bar keeps its normal first row.
        assert!(text.contains("zz-draft-ok"), "no inline draft");
        assert!(text.contains("Enter save"), "no save hint");
        assert!(!text.contains("Comment on"), "prompt still in status bar");
        // Drop back to visual to check the mode tag + comment count row.
        a.commenting = None;
        term.draw(|f| crate::render(f, &mut a)).unwrap();
        let buf = term.backend().buffer().clone();
        let text: String = buf
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(text.contains("--VISUAL--"), "no visual tag");
        assert!(text.contains("[1 comments]"), "no count");
        let blue = buf
            .content()
            .iter()
            .filter(|c| c.bg == Color::Blue)
            .count();
        // Two-line selection: line 0 paints 11 cells (non-cursor row),
        // line 1 paints 3 + cursor cell, so well above 10.
        assert!(blue >= 10, "no blue selection cells in wrap mode: {blue}");
        // Same selection stays visible unwrapped with an h_scroll window.
        a.wrap = false;
        a.h_scroll = 2;
        term.draw(|f| crate::render(f, &mut a)).unwrap();
        let blue = term
            .backend()
            .buffer()
            .content()
            .iter()
            .filter(|c| c.bg == Color::Blue)
            .count();
        assert!(blue > 0, "no blue selection cells with h_scroll");
    }
}
