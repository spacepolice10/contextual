mod app;
mod highlight;
mod picker;
mod search;
mod select;
mod viewer;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{io, path::PathBuf, time::Duration};

#[derive(Parser, Debug)]
#[command(name = "contextual", about = "Minimal terminal text viewer")]
struct Cli {
    /// File to open. If omitted, shows picker for current directory.
    path: Option<PathBuf>,
}

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> Result<Self> {
        crossterm::terminal::enable_raw_mode().context("enable raw mode")?;
        let mut out = io::stdout();
        crossterm::execute!(out, crossterm::terminal::EnterAlternateScreen)
            .context("enter alt screen")?;
        Ok(Self)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut app = match cli.path {
        Some(p) => app::App::load_file(&p, false)?,
        None => {
            let files = picker::list_files(std::path::Path::new("."))?;
            app::App::new_picker(files)
        }
    };
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut term = Terminal::new(backend).context("create terminal")?;
    loop {
        term.draw(|f| render(f, &mut app))?;
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                // Estimate of visible text rows: total height minus 2 status
                // rows (line + bottom padding) minus 2 body border rows.
                app.viewport_h = term.size()?.height.saturating_sub(4) as usize;
                if handle(&mut app, k.code, k.modifiers)? {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn handle(app: &mut app::App, code: KeyCode, mods: KeyModifiers) -> Result<bool> {
    use app::Mode;
    if mods.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return Ok(true);
    }
    match app.mode {
        Mode::Picker => match code {
            KeyCode::Char('q') | KeyCode::Esc => Ok(true),
            KeyCode::Down | KeyCode::Char('j') => {
                app.move_picker(1);
                Ok(false)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.move_picker(-1);
                Ok(false)
            }
            KeyCode::Enter => {
                if let Some(f) = app.files.get(app.picker_index).cloned() {
                    *app = app::App::load_file(&f.path, true)?;
                }
                Ok(false)
            }
            _ => Ok(false),
        },
        Mode::Viewer => {
            if app.searching {
                match code {
                    KeyCode::Esc => {
                        app.cancel_search();
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        if !app.search_matches.is_empty() {
                            let cur = (app.cursor_line, app.cursor_col);
                            let pos = app
                                .search_matches
                                .iter()
                                .position(|&m| m >= cur)
                                .unwrap_or(0);
                            app.search_idx = pos;
                            let (l, c) = app.search_matches[pos];
                            app.cursor_line = l;
                            app.cursor_col = c;
                            let row = crate::viewer::display_row_for_cursor(
                                &app.lines, 80, app.wrap, l, c,
                            );
                            app.scroll = crate::search::scroll_for_match(
                                row,
                                app.scroll,
                                app.viewport_h.max(1),
                                2,
                            );
                        }
                        app.commit_search();
                        return Ok(false);
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                    }
                    KeyCode::Char(c)
                        if !mods.contains(KeyModifiers::CONTROL)
                            && !mods.contains(KeyModifiers::ALT) =>
                    {
                        app.search_query.push(c);
                    }
                    _ => {}
                }
                app.search_matches = crate::search::find_matches(&app.lines, &app.search_query);
                if !app.search_matches.is_empty() {
                    let anchor = app
                        .saved_cursor
                        .unwrap_or((app.cursor_line, app.cursor_col));
                    let pos = app
                        .search_matches
                        .iter()
                        .position(|&m| m >= anchor)
                        .unwrap_or(0);
                    app.search_idx = pos;
                    let (l, c) = app.search_matches[pos];
                    app.cursor_line = l;
                    app.cursor_col = c;
                }
                return Ok(false);
            }
            match code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    // Visual/count state takes priority over search/quit.
                    if code == KeyCode::Esc
                        && (app.pending_count.is_some() || app.visual.is_some())
                    {
                        app.pending_count = None;
                        app.visual = None;
                        return Ok(false);
                    }
                    // Committed search active: first Esc clears it (q still quits).
                    if code == KeyCode::Esc && !app.searching && !app.search_matches.is_empty() {
                        app.cancel_search();
                        return Ok(false);
                    }
                    if app.from_picker && code == KeyCode::Esc {
                        let files = picker::list_files(std::path::Path::new("."))?;
                        *app = app::App::new_picker(files);
                        Ok(false)
                    } else {
                        Ok(true)
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let n = app.pending_count.take().unwrap_or(1) as isize;
                    app.move_cursor_line(n);
                    Ok(false)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let n = app.pending_count.take().unwrap_or(1) as isize;
                    app.move_cursor_line(-n);
                    Ok(false)
                }
                KeyCode::PageDown => {
                    let n = match app.pending_count.take() {
                        Some(c) => c as isize,
                        None => app.viewport_h as isize,
                    };
                    app.move_cursor_line(n);
                    Ok(false)
                }
                KeyCode::PageUp => {
                    let n = match app.pending_count.take() {
                        Some(c) => c as isize,
                        None => app.viewport_h as isize,
                    };
                    app.move_cursor_line(-n);
                    Ok(false)
                }
                KeyCode::Char('f') | KeyCode::Char('F') if mods.contains(KeyModifiers::CONTROL) => {
                    let n = match app.pending_count.take() {
                        Some(c) => c as isize,
                        None => (app.viewport_h / 2).max(1) as isize,
                    };
                    app.move_cursor_line(n);
                    Ok(false)
                }
                KeyCode::Char('b') | KeyCode::Char('B') if mods.contains(KeyModifiers::CONTROL) => {
                    let n = match app.pending_count.take() {
                        Some(c) => c as isize,
                        None => (app.viewport_h / 2).max(1) as isize,
                    };
                    app.move_cursor_line(-n);
                    Ok(false)
                }
                KeyCode::Char('d') | KeyCode::Char('D') if mods.contains(KeyModifiers::CONTROL) => {
                    let n = match app.pending_count.take() {
                        Some(c) => c as isize,
                        None => (app.viewport_h / 2).max(1) as isize,
                    };
                    app.move_cursor_line(n);
                    Ok(false)
                }
                KeyCode::Char('u') | KeyCode::Char('U') if mods.contains(KeyModifiers::CONTROL) => {
                    let n = match app.pending_count.take() {
                        Some(c) => c as isize,
                        None => (app.viewport_h / 2).max(1) as isize,
                    };
                    app.move_cursor_line(-n);
                    Ok(false)
                }
                KeyCode::Char('v')
                    if !mods.contains(KeyModifiers::CONTROL)
                        && !mods.contains(KeyModifiers::ALT) =>
                {
                    app.pending_count = None;
                    if !app.lines.is_empty() {
                        use crate::select::{SelectKind, Selection};
                        match app.visual {
                            Some(sel) if sel.kind == SelectKind::Char => app.visual = None,
                            Some(mut sel) => {
                                sel.kind = SelectKind::Char;
                                app.visual = Some(sel);
                            }
                            None => {
                                app.visual = Some(Selection {
                                    anchor: (app.cursor_line, app.cursor_col),
                                    kind: SelectKind::Char,
                                });
                            }
                        }
                    }
                    Ok(false)
                }
                KeyCode::Char('V')
                    if !mods.contains(KeyModifiers::CONTROL)
                        && !mods.contains(KeyModifiers::ALT) =>
                {
                    app.pending_count = None;
                    if !app.lines.is_empty() {
                        use crate::select::{SelectKind, Selection};
                        match app.visual {
                            Some(sel) if sel.kind == SelectKind::Line => app.visual = None,
                            Some(mut sel) => {
                                sel.kind = SelectKind::Line;
                                app.visual = Some(sel);
                            }
                            None => {
                                app.visual = Some(Selection {
                                    anchor: (app.cursor_line, app.cursor_col),
                                    kind: SelectKind::Line,
                                });
                            }
                        }
                    }
                    Ok(false)
                }
                KeyCode::Char(c @ '1'..='9')
                    if !mods.contains(KeyModifiers::CONTROL)
                        && !mods.contains(KeyModifiers::ALT) =>
                {
                    let d = (c as u8 - b'0') as usize;
                    let n = app.pending_count.unwrap_or(0).saturating_mul(10).saturating_add(d);
                    app.pending_count = Some(n);
                    Ok(false)
                }
                KeyCode::Char('0')
                    if !mods.contains(KeyModifiers::CONTROL)
                        && !mods.contains(KeyModifiers::ALT)
                        && app.pending_count.is_some() =>
                {
                    let n = app.pending_count.unwrap_or(0).saturating_mul(10);
                    app.pending_count = Some(n);
                    Ok(false)
                }
                KeyCode::Char('0') => {
                    app.pending_count = None;
                    app.cursor_col = 0;
                    Ok(false)
                }
                KeyCode::Char('^') => {
                    app.pending_count = None;
                    app.cursor_col = app
                        .lines
                        .get(app.cursor_line)
                        .map(|l| l.chars().position(|c| !c.is_whitespace()).unwrap_or(0))
                        .unwrap_or(0);
                    Ok(false)
                }
                KeyCode::Char('$') => {
                    app.pending_count = None;
                    app.cursor_col = app
                        .lines
                        .get(app.cursor_line)
                        .map(|l| l.chars().count())
                        .unwrap_or(0);
                    Ok(false)
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    match app.pending_count.take() {
                        Some(n) if !app.lines.is_empty() => {
                            // Vim `N g`: jump to 1-based line N, clamped.
                            let line = n.saturating_sub(1).min(app.lines.len() - 1);
                            app.cursor_line = line;
                            let max =
                                app.lines[line].chars().count().min(app.cursor_col);
                            app.cursor_col = max;
                        }
                        _ => {
                            app.cursor_line = 0;
                            app.cursor_col = 0;
                            app.scroll = 0;
                        }
                    }
                    Ok(false)
                }
                KeyCode::End | KeyCode::Char('G') => {
                    match app.pending_count.take() {
                        Some(n) if !app.lines.is_empty() => {
                            // Vim `N G`: jump to 1-based line N, clamped.
                            let line = n.saturating_sub(1).min(app.lines.len() - 1);
                            app.cursor_line = line;
                            let max =
                                app.lines[line].chars().count().min(app.cursor_col);
                            app.cursor_col = max;
                        }
                        _ => {
                            if !app.lines.is_empty() {
                                app.cursor_line = app.lines.len() - 1;
                                app.cursor_col = app.lines[app.cursor_line].chars().count();
                            }
                            app.scroll = usize::MAX;
                        }
                    }
                    Ok(false)
                }
                KeyCode::Char('w') | KeyCode::Char('W') if mods.contains(KeyModifiers::CONTROL) => {
                    app.pending_count = None;
                    app.toggle_wrap();
                    Ok(false)
                }
                KeyCode::Char('w') => {
                    let n = app.pending_count.take().unwrap_or(1);
                    for _ in 0..n {
                        app.move_word_forward(false);
                    }
                    Ok(false)
                }
                KeyCode::Char('W') => {
                    let n = app.pending_count.take().unwrap_or(1);
                    for _ in 0..n {
                        app.move_word_forward(true);
                    }
                    Ok(false)
                }
                KeyCode::Char('e') => {
                    let n = app.pending_count.take().unwrap_or(1);
                    for _ in 0..n {
                        app.move_word_end(false);
                    }
                    Ok(false)
                }
                KeyCode::Char('E') => {
                    let n = app.pending_count.take().unwrap_or(1);
                    for _ in 0..n {
                        app.move_word_end(true);
                    }
                    Ok(false)
                }
                KeyCode::Char('b') if !mods.contains(KeyModifiers::CONTROL) => {
                    let n = app.pending_count.take().unwrap_or(1);
                    for _ in 0..n {
                        app.move_word_back(false);
                    }
                    Ok(false)
                }
                KeyCode::Char('B') if !mods.contains(KeyModifiers::CONTROL) => {
                    let n = app.pending_count.take().unwrap_or(1);
                    for _ in 0..n {
                        app.move_word_back(true);
                    }
                    Ok(false)
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    let n = app.pending_count.take().unwrap_or(1) as isize;
                    app.move_cursor_col(-n);
                    Ok(false)
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    let n = app.pending_count.take().unwrap_or(1) as isize;
                    app.move_cursor_col(n);
                    Ok(false)
                }
                KeyCode::Char('/') => {
                    app.pending_count = None;
                    app.start_search();
                    Ok(false)
                }
                KeyCode::Char('n') => {
                    let n = app.pending_count.take().unwrap_or(1);
                    for _ in 0..n {
                        if !app.search_matches.is_empty() {
                            app.search_idx = (app.search_idx + 1) % app.search_matches.len();
                            let (l, c) = app.search_matches[app.search_idx];
                            app.cursor_line = l;
                            app.cursor_col = c;
                        }
                    }
                    Ok(false)
                }
                KeyCode::Char('N') => {
                    let n = app.pending_count.take().unwrap_or(1);
                    for _ in 0..n {
                        if !app.search_matches.is_empty() {
                            app.search_idx = (app.search_idx + app.search_matches.len() - 1)
                                % app.search_matches.len();
                            let (l, c) = app.search_matches[app.search_idx];
                            app.cursor_line = l;
                            app.cursor_col = c;
                        }
                    }
                    Ok(false)
                }
                _ => {
                    app.pending_count = None;
                    Ok(false)
                }
            }
        }
    }
}

/// Repaint exact-match ranges with vim-like Search style (yellow bg).
/// Ranges are chunk-relative char offsets into the concatenated spans.
fn paint_search_ranges(spans: Vec<Span<'static>>, ranges: &[(usize, usize)]) -> Vec<Span<'static>> {
    let bg = Style::default().bg(Color::Yellow).fg(Color::Black);
    let mut cells: Vec<(char, Style)> = Vec::new();
    for s in &spans {
        for c in s.content.chars() {
            cells.push((c, s.style));
        }
    }
    for &(a, b) in ranges {
        for i in a.min(cells.len())..b.min(cells.len()) {
            cells[i].1 = bg;
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

fn viewer_hint(searching: bool) -> &'static str {
    if searching {
        "Enter ok · Esc cancel"
    } else {
        "j/k move · ^F/^B/^D/^U half · w/b/e word · 0^$ line · / n/N · g/G · ^W wrap · q quit"
    }
}

/// Left status + right-aligned hint padded to `width` chars (char count,
/// not bytes). Hint keeps a subtle style; truncates on narrow widths.
fn status_line(width: usize, left: &str, right: &str) -> Line<'static> {
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

fn render(f: &mut ratatui::Frame, app: &mut app::App) {
    use app::Mode;
    let area = f.area();
    match app.mode {
        Mode::Picker => {
            let items: Vec<ListItem> = app
                .files
                .iter()
                .map(|e| {
                    ListItem::new(Line::from(vec![Span::raw(format!(
                        "{}  ({}b)",
                        e.name, e.size
                    ))]))
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" contextual — pick a file (Enter/q) "),
                )
                .highlight_style(Style::default().bg(Color::DarkGray));
            if app.files.is_empty() {
                let p = Paragraph::new("No files in this directory — press q to quit.").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" contextual "),
                );
                f.render_widget(p, area);
            } else {
                use ratatui::widgets::ListState;
                let mut st = ListState::default();
                st.select(Some(app.picker_index));
                f.render_stateful_widget(list, area, &mut st);
            }
        }
        Mode::Viewer => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(2)])
                .split(area);
            let gutter = app.lines.len().to_string().len().max(4) + 1;
            let (text_w, vh) =
                viewer::content_size(chunks[0].width as usize, chunks[0].height as usize, gutter);
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
                // Highlight the cursor cell on the cursor's display row.
                let is_cursor_row = abs_row == cursor_row;
                let body_span = if !is_cursor_row {
                    if hl_row {
                        windowed
                    } else {
                        vec![Span::raw(shown)]
                    }
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
                        let chars: Vec<char> = shown.chars().collect();
                        let before: String = chars[..rel_col].iter().collect();
                        let cur: String = chars[rel_col..rel_col + 1].iter().collect();
                        let after: String = chars[rel_col + 1..].iter().collect();
                        vec![
                            Span::raw(before),
                            Span::styled(cur, cursor_style),
                            Span::raw(after),
                        ]
                    }
                };
                let mut spans = vec![Span::styled(
                    format!("{:>width$} ", lidx + 1, width = gutter - 1),
                    Style::default().fg(Color::DarkGray),
                )];
                spans.extend(body_span);
                text.push(Line::from(spans));
            }
            let body = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(format!(" {} ", app.filename)),
            );
            f.render_widget(body, chunks[0]);
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
            let bar_w = chunks[1].width as usize;
            let first = status_line(bar_w, &s, viewer_hint(app.searching));
            let bar = Paragraph::new(vec![first, Line::from(Span::raw(""))]);
            f.render_widget(bar, chunks[1]);
        }
    }
}

#[cfg(test)]
mod handle_tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    fn viewer() -> app::App {
        app::App::load_file(std::path::Path::new("Cargo.toml"), false).unwrap()
    }
    #[test]
    fn slash_enters_search_and_esc_restores() {
        let mut a = viewer();
        handle(&mut a, KeyCode::Char('/'), KeyModifiers::NONE).unwrap();
        assert!(a.searching);
        handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(!a.searching);
    }
    #[test]
    fn esc_clears_committed_search() {
        let mut a = viewer();
        handle(&mut a, KeyCode::Char('/'), KeyModifiers::NONE).unwrap();
        for c in "package".chars() {
            handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap();
        }
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(!a.search_matches.is_empty());
        let quit = handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(!quit);
        assert!(a.search_query.is_empty());
        assert!(a.search_matches.is_empty());
    }
    #[test]
    fn typing_runs_matcher_and_n_advances() {
        let mut a = viewer();
        handle(&mut a, KeyCode::Char('/'), KeyModifiers::NONE).unwrap();
        for c in "package".chars() {
            handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap();
        }
        assert!(!a.search_matches.is_empty());
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(!a.searching);
        let first = a.search_idx;
        handle(&mut a, KeyCode::Char('n'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.search_idx, (first + 1) % a.search_matches.len());
    }
    #[test]
    fn scroll_landing_uses_display_row() {
        let lines = vec!["abcdef".to_string(), "xy".to_string()];
        let row = crate::viewer::display_row_for_cursor(&lines, 2, true, 0, 3);
        assert_eq!(row, 1);
        assert_eq!(crate::search::scroll_for_match(row, 0, 5, 2), 0);
    }
    #[test]
    fn ctrl_f_half_page_down() {
        let mut a = viewer();
        a.viewport_h = 20;
        a.cursor_line = 0;
        handle(&mut a, KeyCode::Char('f'), KeyModifiers::CONTROL).unwrap();
        assert_eq!(a.cursor_line, 10);
    }
    #[test]
    fn ctrl_b_half_page_up() {
        let mut a = viewer();
        a.viewport_h = 20;
        a.cursor_line = 20;
        handle(&mut a, KeyCode::Char('b'), KeyModifiers::CONTROL).unwrap();
        assert_eq!(a.cursor_line, 10);
    }
    #[test]
    fn ctrl_d_half_page_down() {
        let mut a = viewer();
        a.viewport_h = 20;
        a.cursor_line = 0;
        handle(&mut a, KeyCode::Char('d'), KeyModifiers::CONTROL).unwrap();
        assert_eq!(a.cursor_line, 10);
    }
    #[test]
    fn ctrl_u_half_page_up() {
        let mut a = viewer();
        a.viewport_h = 20;
        a.cursor_line = 20;
        handle(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL).unwrap();
        assert_eq!(a.cursor_line, 10);
    }
    #[test]
    fn zero_moves_to_col_start() {
        let mut a = viewer();
        a.lines = vec!["hello".to_string()];
        a.cursor_line = 0;
        a.cursor_col = 3;
        handle(&mut a, KeyCode::Char('0'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.cursor_col, 0);
    }
    #[test]
    fn caret_moves_to_first_non_blank() {
        let mut a = viewer();
        a.lines = vec!["   hello".to_string()];
        a.cursor_line = 0;
        a.cursor_col = 7;
        handle(&mut a, KeyCode::Char('^'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.cursor_col, 3);
    }
    #[test]
    fn dollar_moves_to_line_end() {
        let mut a = viewer();
        a.lines = vec!["hello".to_string()];
        a.cursor_line = 0;
        a.cursor_col = 0;
        handle(&mut a, KeyCode::Char('$'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.cursor_col, 5);
    }
    #[test]
    fn ctrl_w_toggles_wrap() {
        let mut a = viewer();
        assert!(a.wrap);
        handle(&mut a, KeyCode::Char('w'), KeyModifiers::CONTROL).unwrap();
        assert!(!a.wrap);
    }
    #[test]
    fn w_moves_to_next_word_start() {
        let mut a = viewer();
        a.lines = vec!["foo bar".to_string()];
        a.cursor_line = 0;
        a.cursor_col = 0;
        handle(&mut a, KeyCode::Char('w'), KeyModifiers::NONE).unwrap();
        assert_eq!((a.cursor_line, a.cursor_col), (0, 4));
    }
    #[test]
    fn e_moves_to_word_end() {
        let mut a = viewer();
        a.lines = vec!["foo bar".to_string()];
        a.cursor_line = 0;
        a.cursor_col = 0;
        handle(&mut a, KeyCode::Char('e'), KeyModifiers::NONE).unwrap();
        assert_eq!((a.cursor_line, a.cursor_col), (0, 2));
    }
    #[test]
    fn visual_count_word_motion() {
        let mut a = viewer();
        a.lines = vec!["foo bar baz".to_string()];
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        for c in ['3', 'w'] { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
        assert_eq!((a.cursor_line, a.cursor_col), (0, 8));
    }
    #[test]
    fn visual_5j_and_esc() {
        let mut a = viewer();
        a.lines = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string(), "e".to_string(), "f".to_string()];
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        for c in ['5', 'j'] { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
        assert_eq!(a.cursor_line, 5);
        handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(a.visual.is_none());
    }
    #[test]
    fn visual_anchor_stays_and_v_exits() {
        let mut a = viewer();
        a.lines = vec!["foo bar baz".to_string()];
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        let anchor = a.visual.unwrap().anchor;
        assert_eq!(anchor, (0, 0));
        handle(&mut a, KeyCode::Char('w'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.visual.unwrap().anchor, (0, 0));
        assert_eq!((a.cursor_line, a.cursor_col), (0, 4));
        // Same-kind `v` exits visual.
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        assert!(a.visual.is_none());
    }
    #[test]
    fn bare_zero_goes_to_col_zero_and_ten_j_moves_ten() {
        let mut a = viewer();
        a.lines = (0..12).map(|i| format!("line {i}")).collect();
        a.cursor_col = 3;
        handle(&mut a, KeyCode::Char('0'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.cursor_col, 0);
        assert!(a.pending_count.is_none());
        for c in ['1', '0', 'j'] {
            handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap();
        }
        assert_eq!(a.cursor_line, 10);
        assert!(a.pending_count.is_none());
    }
    #[test]
    fn esc_clears_pending_without_quitting() {
        let mut a = viewer();
        a.lines = vec!["a".to_string(), "b".to_string()];
        handle(&mut a, KeyCode::Char('5'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.pending_count, Some(5));
        let quit = handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(!quit);
        assert!(a.pending_count.is_none());
    }
    #[test]
    fn b_moves_to_prev_word_start() {
        let mut a = viewer();
        a.lines = vec!["foo bar".to_string()];
        a.cursor_line = 0;
        a.cursor_col = 4;
        handle(&mut a, KeyCode::Char('b'), KeyModifiers::NONE).unwrap();
        assert_eq!((a.cursor_line, a.cursor_col), (0, 0));
    }
}

#[cfg(test)]
mod status_tests {
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
}
