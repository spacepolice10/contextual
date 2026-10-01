use crate::file_picker::{picker_preview_visible, picker_prompt_line, read_preview_lines};
use crate::ui::paint::{paint_ranges, paint_search_ranges};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Picker screen: prompt row + file list + live preview pane.
pub fn render_picker(f: &mut Frame, app: &mut crate::app::App, area: Rect) {
            let show_preview = picker_preview_visible(area.width as usize);
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(area);
            let prompt = picker_prompt_line(
                &app.picker.query,
                app.picker.filtered.len(),
                app.files.len(),
                app.picker.truncated,
            );
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw(prompt),
                    Span::styled(
                        "▌",
                        Style::default().bg(Color::DarkGray).fg(Color::White),
                    ),
                ])),
                rows[0],
            );
            let (list_area, preview_area) = if show_preview {
                let h = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(rows[1]);
                (h[0], Some(h[1]))
            } else {
                (rows[1], None)
            };
            let list_h = list_area.height.saturating_sub(2) as usize;
            let row_w = list_area.width.saturating_sub(2) as usize;
            let total = app.picker.filtered.len();
            let sel = app.picker.selected.min(total.saturating_sub(1));
            // Window keeping the selection visible, biased to context below.
            let start = if total <= list_h.max(1) || list_h == 0 {
                0
            } else {
                sel.saturating_sub(list_h / 2).min(total - list_h)
            };
            let end = (start + list_h.max(1)).min(total);
            let dim = Style::default().fg(Color::DarkGray);
            let sel_style = Style::default().bg(Color::DarkGray);
            let mut items = Vec::new();
            if total == 0 {
                let msg = if app.picker.query.is_empty() {
                    "No files in this directory — press q to quit."
                } else {
                    "No matches — keep typing or Esc to clear."
                };
                items.push(ListItem::new(Line::from(Span::styled(msg, dim))));
            } else {
                for (i, m) in app.picker.filtered[start..end].iter().enumerate() {
                    if let Some(entry) = app.files.get(m.entry_idx) {
                        let full = crate::file_picker::display_path(entry);
                        let shown: String = full.chars().take(row_w.max(1)).collect();
                        let n = shown.chars().count();
                        let mut spans = vec![Span::raw(shown)];
                        if start + i == sel {
                            spans = paint_ranges(spans, &[(0, n)], sel_style);
                        }
                        let ranges: Vec<(usize, usize)> = m
                            .cols
                            .iter()
                            .filter(|&&c| c < n)
                            .map(|&c| (c, c + 1))
                            .collect();
                        if !ranges.is_empty() {
                            spans = paint_search_ranges(spans, &ranges);
                        }
                        items.push(ListItem::new(Line::from(spans)));
                    }
                }
            }
            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" contextual — pick a file "),
            );
            f.render_widget(list, list_area);
            if let Some(preview) = preview_area {
                let inner_w = preview.width.saturating_sub(2) as usize;
                let inner_h = preview.height.saturating_sub(2) as usize;
                let hit = app
                    .picker
                    .filtered
                    .get(app.picker.selected)
                    .and_then(|m| app.files.get(m.entry_idx));
                let (title, body_rows) = match hit {
                    None => (
                        " preview ".to_string(),
                        vec![Line::from(Span::styled("No matches", dim))],
                    ),
                    Some(entry) => {
                        let shown = crate::file_picker::display_path(entry);
                        let (lines, note) = read_preview_lines(&entry.path, 200);
                        let text = lines.join("\n");
                        let title = match note {
                            Some(n) => format!(" {shown} {n} "),
                            None => format!(" {shown} "),
                        };
                        // Styled source per logical line; length mismatch
                        // falls back to plain rows (never drops text).
                        let styled: Vec<Vec<Span<'static>>> =
                            match crate::highlight::detect(&entry.path).and_then(|l| {
                                crate::highlight::highlight_file(&text, l, app.theme)
                            }) {
                                Some(h) if h.len() == lines.len() => h,
                                _ => lines
                                    .iter()
                                    .map(|l| vec![Span::raw(l.clone())])
                                    .collect(),
                            };
                        let display =
                            crate::viewer::build_display_lines(&lines, inner_w.max(1), true);
                        app.picker.preview_scroll = crate::viewer::clamp_scroll(
                            app.picker.preview_scroll,
                            display.len(),
                            inner_h,
                        );
                        let pend =
                            (app.picker.preview_scroll + inner_h).min(display.len());
                        let w = inner_w.max(1);
                        let mut prev: Option<usize> = None;
                        let mut k: usize = 0;
                        let mut body_rows = Vec::new();
                        for (lidx, _) in &display[app.picker.preview_scroll..pend] {
                            match prev {
                                Some(p) if p == *lidx => {
                                    k += 1;
                                }
                                _ => {
                                    k = 0;
                                }
                            }
                            prev = Some(*lidx);
                            let src: &[Span<'static>] =
                                styled.get(*lidx).map(|v| v.as_slice()).unwrap_or(&[]);
                            body_rows.push(Line::from(
                                crate::highlight::slice_spans(src, k * w, w),
                            ));
                        }
                        (title, body_rows)
                    }
                };
                let panel = Paragraph::new(body_rows).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(title),
                );
                f.render_widget(panel, preview);
            }
}
