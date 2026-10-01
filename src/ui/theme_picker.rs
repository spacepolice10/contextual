use crate::file_picker::picker_preview_visible;
use crate::ui::paint::paint_ranges;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Theme picker screen: theme list + live preview of the current file.
/// Preview reuses `app.highlighted`, which the picker moves keep in sync
/// with the hovered theme (never drops text: length mismatch -> plain).
pub fn render_theme_picker(f: &mut Frame, app: &mut crate::app::App, area: Rect) {
    let show_preview = picker_preview_visible(area.width as usize);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " themes  Enter apply · Esc cancel ",
            Style::default().fg(Color::DarkGray),
        ))),
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
    let themes = crate::highlight::Theme::all();
    let sel = app
        .theme_picker
        .as_ref()
        .map(|p| p.selected)
        .unwrap_or(0)
        .min(themes.len().saturating_sub(1));
    let list_h = list_area.height.saturating_sub(2) as usize;
    let start = if themes.len() <= list_h.max(1) || list_h == 0 {
        0
    } else {
        sel.saturating_sub(list_h / 2).min(themes.len() - list_h)
    };
    let end = (start + list_h.max(1)).min(themes.len());
    let dim = Style::default().fg(Color::DarkGray);
    let sel_style = Style::default().bg(Color::DarkGray);
    let mut items = Vec::new();
    for (i, t) in themes[start..end].iter().enumerate() {
        let mut spans = vec![Span::raw(t.name().to_string())];
        if start + i == sel {
            let n = t.name().chars().count();
            spans = paint_ranges(spans, &[(0, n)], sel_style);
        }
        items.push(ListItem::new(Line::from(spans)));
    }
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" themes "),
    );
    f.render_widget(list, list_area);
    if let Some(preview) = preview_area {
        let inner_w = preview.width.saturating_sub(2) as usize;
        let inner_h = preview.height.saturating_sub(2) as usize;
        let w = inner_w.max(1);
        let display = crate::viewer::build_display_lines(&app.lines, w, true);
        let end = inner_h.min(display.len());
        let mut prev: Option<usize> = None;
        let mut k: usize = 0;
        let mut body_rows = Vec::new();
        for (lidx, _) in &display[..end] {
            match prev {
                Some(p) if p == *lidx => k += 1,
                _ => k = 0,
            }
            prev = Some(*lidx);
            let src: &[Span<'static>] =
                app.highlighted.as_ref().and_then(|h| h.get(*lidx)).map(|v| v.as_slice()).unwrap_or(&[]);
            body_rows.push(Line::from(crate::highlight::slice_spans(src, k * w, w)));
        }
        if body_rows.is_empty() {
            body_rows.push(Line::from(Span::styled("No preview", dim)));
        }
        let title = format!(" {} [{}] ", app.filename, themes[sel].name());
        let panel = Paragraph::new(body_rows).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        );
        f.render_widget(panel, preview);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn theme_picker_lists_builtin_themes() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut a =
            crate::app::App::load_file(std::path::Path::new("Cargo.toml"), false).unwrap();
        a.open_theme_picker();
        let backend = TestBackend::new(100, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| crate::ui::theme_picker::render_theme_picker(f, &mut a, f.area()))
            .unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        for name in ["catppuccin", "nord", "dracula"] {
            assert!(text.contains(name), "missing theme {name}");
        }
    }
}
