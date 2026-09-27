mod app;
mod cli;
mod highlight;
mod picker;
mod search;
mod select;
mod tui;
mod ui;
mod viewer;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use crate::ui::paint::{paint_search_ranges, paint_selection_ranges};
use crate::ui::sidebar::sidebar_lines;
use crate::ui::status::{comment_prompt, comments_tag, status_line, viewer_hint, visual_tag};
use std::{io, time::Duration};

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let mut app = match cli.path {
        Some(p) => app::App::load_file(&p, false)?,
        None => {
            let (files, truncated) = picker::discover_files(std::path::Path::new("."));
            app::App::new_picker(files, truncated)
        }
    };
    let _guard = tui::TerminalGuard::enter()?;
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

/// Max accumulated count prefix (`v3w`, `5j`); mirrors `search::MAX_MATCHES`
/// style so huge digit runs can't hang repeat loops or wrap `as isize`.
const MAX_COUNT: usize = 10_000;

fn handle(app: &mut app::App, code: KeyCode, mods: KeyModifiers) -> Result<bool> {
    use app::Mode;
    if mods.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return Ok(true);
    }
    match app.mode {
        Mode::Picker => {
            let ctrl = mods.contains(KeyModifiers::CONTROL);
            let alt = mods.contains(KeyModifiers::ALT);
            match code {
                KeyCode::Char('u') if ctrl => {
                    app.picker_clear();
                    Ok(false)
                }
                KeyCode::Char('j') | KeyCode::Char('n') if ctrl => {
                    app.picker_move(1);
                    Ok(false)
                }
                KeyCode::Char('k') | KeyCode::Char('p') if ctrl => {
                    app.picker_move(-1);
                    Ok(false)
                }
                KeyCode::Down => {
                    app.picker_move(1);
                    Ok(false)
                }
                KeyCode::Up => {
                    app.picker_move(-1);
                    Ok(false)
                }
                KeyCode::Backspace => {
                    app.picker.query.pop();
                    app.picker_recompute();
                    Ok(false)
                }
                KeyCode::Enter => {
                    let hit = app
                        .picker
                        .filtered
                        .get(app.picker.selected)
                        .map(|m| m.entry_idx);
                    match hit.and_then(|i| app.files.get(i).cloned()) {
                        Some(f) => {
                            *app = app::App::load_file(&f.path, true)?;
                            Ok(false)
                        }
                        None => Ok(false),
                    }
                }
                KeyCode::Esc => {
                    if app.picker.query.is_empty() {
                        Ok(true)
                    } else {
                        app.picker_clear();
                        Ok(false)
                    }
                }
                // `q` quits a fresh picker but types once filtering, so
                // queries containing `q` (e.g. `sqlite`) stay searchable.
                KeyCode::Char('q') if app.picker.query.is_empty() => Ok(true),
                KeyCode::Char(c) if !ctrl && !alt => {
                    app.picker.query.push(c);
                    app.picker_recompute();
                    Ok(false)
                }
                _ => Ok(false),
            }
        }
        Mode::Viewer => {
            // Comment draft input: keystrokes edit `draft`, Enter saves to
            // the memory buffer, Esc drops back to visual (selection kept).
            if app.commenting.is_some() {
                match code {
                    KeyCode::Esc => {
                        app.commenting = None;
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        if let Some(p) = app.commenting.take() {
                            let id = app.comments.len();
                            app.comments.push(app::Comment {
                                id,
                                file: app.filename.clone(),
                                start: p.start,
                                end: p.end,
                                snippet: p.snippet,
                                note: p.draft,
                            });
                        }
                        app.visual = None;
                        app.pending_count = None;
                        return Ok(false);
                    }
                    KeyCode::Backspace => {
                        if let Some(p) = app.commenting.as_mut() {
                            p.draft.pop();
                        }
                        return Ok(false);
                    }
                    KeyCode::Char(c)
                        if !mods.contains(KeyModifiers::CONTROL)
                            && !mods.contains(KeyModifiers::ALT) =>
                    {
                        if let Some(p) = app.commenting.as_mut() {
                            p.draft.push(c);
                        }
                        return Ok(false);
                    }
                    _ => return Ok(false),
                }
            }
            // Text-object pending state: `i`/`a` in visual (below) armed
            // this; a delimiter char resolves via `find_delim_pair`, any
            // other key cancels back to normal handling.
            if let Some(inner) = app.pending_object.take() {
                let pair = match code {
                    KeyCode::Char(c)
                        if !mods.contains(KeyModifiers::CONTROL)
                            && !mods.contains(KeyModifiers::ALT) =>
                    {
                        match c {
                            '(' | ')' => Some(('(', ')')),
                            '[' | ']' => Some(('[', ']')),
                            '{' | '}' => Some(('{', '}')),
                            '<' | '>' => Some(('<', '>')),
                            '"' | '\'' | '`' => Some((c, c)),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if let Some((open, close)) = pair {
                    let cursor = (app.cursor_line, app.cursor_col);
                    match crate::select::find_delim_pair(&app.lines, cursor, open, close, inner)
                    {
                        Some((s, e)) => {
                            if let Some(sel) = app.visual.as_mut() {
                                sel.anchor = s;
                            }
                            app.cursor_line = e.0;
                            app.cursor_col = e.1;
                            app.status_note = None;
                        }
                        None => {
                            app.status_note = Some("no match".to_string());
                        }
                    }
                    return Ok(false);
                }
            }
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
                        let (files, truncated) = picker::discover_files(std::path::Path::new("."));
                        *app = app::App::new_picker(files, truncated);
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
                    let n = app
                        .pending_count
                        .unwrap_or(0)
                        .saturating_mul(10)
                        .saturating_add(d)
                        .min(MAX_COUNT);
                    app.pending_count = Some(n);
                    Ok(false)
                }
                KeyCode::Char('0')
                    if !mods.contains(KeyModifiers::CONTROL)
                        && !mods.contains(KeyModifiers::ALT)
                        && app.pending_count.is_some() =>
                {
                    let n = app.pending_count.unwrap_or(0).saturating_mul(10).min(MAX_COUNT);
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
                KeyCode::Home => {
                    app.pending_count = None;
                    app.cursor_line = 0;
                    app.cursor_col = 0;
                    app.scroll = 0;
                    Ok(false)
                }
                KeyCode::Char('g') => {
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
                KeyCode::End => {
                    app.pending_count = None;
                    if !app.lines.is_empty() {
                        app.cursor_line = app.lines.len() - 1;
                        app.cursor_col = app.lines[app.cursor_line].chars().count();
                    }
                    app.scroll = usize::MAX;
                    Ok(false)
                }
                KeyCode::Char('G') => {
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
                KeyCode::Char(c @ ('i' | 'a'))
                    if !mods.contains(KeyModifiers::CONTROL)
                        && !mods.contains(KeyModifiers::ALT)
                        && app.visual.is_some() =>
                {
                    // Vim `i`/`a` text-object: arm delimiter pending state;
                    // the next delimiter key selects via `find_delim_pair`.
                    app.pending_count = None;
                    app.pending_object = Some(c == 'i');
                    Ok(false)
                }
                KeyCode::Enter => {
                    // With an active selection, stash the snippet + span and
                    // open the draft input; with no visual this is a noop
                    // (also covers the empty-file case, where `v` never arms).
                    app.pending_count = None;
                    if let Some(sel) = app.visual {
                        if !app.lines.is_empty() {
                            let cursor = (app.cursor_line, app.cursor_col);
                            let (start, end) =
                                crate::select::normalize(sel.anchor, cursor, sel.kind);
                            let snippet =
                                crate::select::extract_text(&app.lines, start, end, sel.kind);
                            app.commenting = Some(app::PendingComment {
                                snippet,
                                start,
                                end,
                                draft: String::new(),
                            });
                        }
                    }
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
                KeyCode::Char('C')
                    if !mods.contains(KeyModifiers::CONTROL)
                        && !mods.contains(KeyModifiers::ALT) =>
                {
                    app.pending_count = None;
                    app.show_sidebar = !app.show_sidebar;
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

/// Gutter number (vim `relativenumber` style): the cursor's own line
/// shows its absolute 1-based number, every other line shows the
/// distance from the cursor.
fn gutter_number(lidx: usize, cursor_line: usize) -> usize {
    if lidx == cursor_line {
        lidx + 1
    } else {
        lidx.abs_diff(cursor_line)
    }
}
fn render(f: &mut ratatui::Frame, app: &mut app::App) {
    use app::Mode;
    let area = f.area();
    match app.mode {
        Mode::Picker => crate::ui::picker::render_picker(f, app, area),
        Mode::Viewer => {
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
            // While typing a note, row 1 is the input
            // (`Comment on <file:line>: <draft>▌`) and row 2 the confirm
            // hint; otherwise row 1 is the status line.
            let rows = if let Some(p) = &app.commenting {
                let mut first = vec![Span::raw(comment_prompt(
                    &app.filename,
                    p.start.0 + 1,
                    &p.draft,
                ))];
                first.push(Span::styled(
                    "▌",
                    Style::default().bg(Color::DarkGray).fg(Color::White),
                ));
                vec![
                    Line::from(first),
                    Line::from(Span::styled(
                        "Enter save · Esc cancel",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            } else {
                vec![
                    status_line(bar_w, &s, viewer_hint(app.searching)),
                    Line::from(Span::raw("")),
                ]
            };
            let bar = Paragraph::new(rows);
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
    fn picker_entry(path: &str) -> crate::picker::FileEntry {
        crate::picker::FileEntry {
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            path: std::path::PathBuf::from(path),
            size: 0,
        }
    }
    fn picker_app() -> app::App {
        app::App::new_picker(vec![picker_entry("b.txt"), picker_entry("a.txt")], false)
    }
    #[test]
    fn picker_typing_filters_and_ctrl_u_clears() {
        let mut a = picker_app();
        assert_eq!(a.picker.filtered.len(), 2);
        handle(&mut a, KeyCode::Char('a'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.picker.query, "a");
        assert_eq!(a.picker.filtered.len(), 1);
        handle(&mut a, KeyCode::Char('u'), KeyModifiers::CONTROL).unwrap();
        assert!(a.picker.query.is_empty());
        assert_eq!(a.picker.filtered.len(), 2);
    }
    #[test]
    fn picker_ctrl_jk_move_and_wrap() {
        let mut a = picker_app();
        handle(&mut a, KeyCode::Char('j'), KeyModifiers::CONTROL).unwrap();
        assert_eq!(a.picker.selected, 1);
        handle(&mut a, KeyCode::Char('j'), KeyModifiers::CONTROL).unwrap();
        assert_eq!(a.picker.selected, 0);
        handle(&mut a, KeyCode::Char('k'), KeyModifiers::CONTROL).unwrap();
        assert_eq!(a.picker.selected, 1);
    }
    #[test]
    fn picker_esc_clears_first_then_quits() {
        let mut a = picker_app();
        handle(&mut a, KeyCode::Char('a'), KeyModifiers::NONE).unwrap();
        let quit = handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(!quit);
        assert!(a.picker.query.is_empty());
        let quit = handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(quit);
    }
    #[test]
    fn picker_enter_loads_selected() {
        let dir = std::env::temp_dir().join(format!(
            "ctx_enter_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        std::fs::write(&first, "first\n").unwrap();
        std::fs::write(&second, "second\n").unwrap();
        let mut a = app::App::new_picker(
            vec![
                crate::picker::FileEntry {
                    name: "first.txt".to_string(),
                    path: first,
                    size: 6,
                },
                crate::picker::FileEntry {
                    name: "second.txt".to_string(),
                    path: second,
                    size: 7,
                },
            ],
            false,
        );
        // Select the second hit, not `files[0]`: Enter must follow selection.
        a.picker.selected = 1;
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert_eq!(a.mode, app::Mode::Viewer);
        assert!(a.filename.contains("second.txt"));
        assert!(a.from_picker);
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn picker_enter_empty_is_noop() {
        let mut a = picker_app();
        handle(&mut a, KeyCode::Char('z'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::Char('z'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::Char('z'), KeyModifiers::NONE).unwrap();
        assert!(a.picker.filtered.is_empty());
        let quit = handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(!quit);
        assert_eq!(a.mode, app::Mode::Picker);
    }
    #[test]
    fn esc_from_viewer_returns_to_picker() {
        // Recursive discovery: some entry descends into a subdir (two `/`
        // in `./src/main.rs`); the old flat lister only yields one.
        let mut a = viewer();
        a.from_picker = true;
        let quit = handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(!quit);
        assert_eq!(a.mode, app::Mode::Picker);
        assert!(!a.files.is_empty());
        assert!(
            a.files.iter().any(|f| crate::picker::display_path(f)
                .chars()
                .filter(|&c| c == '/')
                .count()
                >= 2),
            "expected recursive entries"
        );
        assert_eq!(a.picker.filtered.len(), a.files.len());
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
    fn count_home_stays_top_and_end_goes_bottom() {
        let mut a = viewer();
        a.lines = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        a.cursor_line = 1;
        handle(&mut a, KeyCode::Char('5'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::Home, KeyModifiers::NONE).unwrap();
        assert_eq!((a.cursor_line, a.cursor_col), (0, 0));
        assert!(a.pending_count.is_none());
        a.cursor_line = 0;
        handle(&mut a, KeyCode::Char('5'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::End, KeyModifiers::NONE).unwrap();
        assert_eq!(a.cursor_line, 2);
        assert!(a.pending_count.is_none());
        // Counts still apply to `g`/`G`.
        handle(&mut a, KeyCode::Char('2'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::Char('G'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.cursor_line, 1);
    }
    #[test]
    fn huge_count_is_capped_and_does_not_hang() {
        let mut a = viewer();
        a.lines = vec!["foo bar".to_string(), "baz".to_string()];
        for c in "9999999999".chars() {
            handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap();
        }
        assert_eq!(a.pending_count, Some(10_000));
        handle(&mut a, KeyCode::Char('w'), KeyModifiers::NONE).unwrap();
        assert!(a.pending_count.is_none());
        assert!((a.cursor_line, a.cursor_col) <= (1, 3));
        for c in "9999999999".chars() {
            handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap();
        }
        handle(&mut a, KeyCode::Char('j'), KeyModifiers::NONE).unwrap();
        assert_eq!(a.cursor_line, 1);
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
    #[test]
    fn vi_quote_then_enter_saves_comment() {
        let mut a = viewer();
        a.lines = vec!["a \"hi\" b".to_string()];
        a.cursor_col = 4;
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        for c in ['i', '"'] { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(a.commenting.is_some());
        for c in "ok".chars() { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert_eq!(a.comments.len(), 1);
        assert_eq!(a.comments[0].snippet, "hi");
    }
    #[test]
    fn delim_miss_keeps_selection() {
        let mut a = viewer();
        a.lines = vec!["no quotes".to_string()];
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        for c in ['i', '"'] { handle(&mut a, KeyCode::Char(c), KeyModifiers::NONE).unwrap(); }
        assert!(a.visual.is_some());
        assert!(a.commenting.is_none());
        assert_eq!(a.status_note.as_deref(), Some("no match"));
    }
    #[test]
    fn enter_noop_without_visual_and_on_empty_file() {
        let mut a = viewer();
        a.lines = vec!["hello".to_string()];
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(a.commenting.is_none());
        assert!(a.comments.is_empty());
        // Empty file: `v` never arms visual, so `Enter` stays a noop.
        a.lines = vec![];
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        assert!(a.visual.is_none());
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(a.commenting.is_none());
        assert!(a.comments.is_empty());
    }
    #[test]
    fn empty_note_is_saved_with_file_ref() {
        let mut a = viewer();
        a.lines = vec!["hello".to_string()];
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::Char('e'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(a.commenting.is_some());
        // No typing: empty draft still saves.
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert_eq!(a.comments.len(), 1);
        assert_eq!(a.comments[0].note, "");
        assert_eq!(a.comments[0].file, a.filename);
        assert!(a.visual.is_none());
        assert!(a.commenting.is_none());
    }
    #[test]
    fn c_toggles_sidebar() {
        let mut a = viewer();
        assert!(!a.show_sidebar);
        handle(&mut a, KeyCode::Char('C'), KeyModifiers::NONE).unwrap();
        assert!(a.show_sidebar);
        handle(&mut a, KeyCode::Char('C'), KeyModifiers::NONE).unwrap();
        assert!(!a.show_sidebar);
    }
    #[test]
    fn esc_drops_commenting_back_to_visual() {
        let mut a = viewer();
        a.lines = vec!["hello".to_string()];
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::Enter, KeyModifiers::NONE).unwrap();
        assert!(a.commenting.is_some());
        let quit = handle(&mut a, KeyCode::Esc, KeyModifiers::NONE).unwrap();
        assert!(!quit);
        assert!(a.commenting.is_none());
        assert!(a.visual.is_some());
        assert!(a.comments.is_empty());
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;
    fn viewer() -> app::App {
        app::App::load_file(std::path::Path::new("Cargo.toml"), false).unwrap()
    }
    #[test]
    fn status_shows_visual_and_comment_count() {
        let mut a = viewer();
        a.lines = vec!["hi".to_string()];
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        assert!(viewer_hint(false).contains("C"));
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
    fn render_smoke_visual_sidebar_commenting() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut a = viewer();
        a.lines = vec!["hello world".to_string(), "second line".to_string()];
        a.lang = None;
        a.highlighted = None;
        handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::Char('e'), KeyModifiers::NONE).unwrap();
        handle(&mut a, KeyCode::Char('j'), KeyModifiers::NONE).unwrap();
        assert!(a.visual.is_some());
        a.show_sidebar = true;
        a.comments.push(app::Comment {
            id: 0,
            file: "f.rs".to_string(),
            start: (0, 0),
            end: (0, 4),
            snippet: "hello".to_string(),
            note: "n".to_string(),
        });
        a.commenting = Some(app::PendingComment {
            snippet: "hello".to_string(),
            start: (0, 0),
            end: (0, 5),
            draft: "ok".to_string(),
        });
        let backend = TestBackend::new(80, 24);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &mut a)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(text.contains("comments (C)"), "no sidebar title");
        assert!(text.contains("Comment on"), "no input row");
        // Input row replaces the status line while commenting; drop back
        // to visual to check the mode tag + comment count row.
        a.commenting = None;
        term.draw(|f| render(f, &mut a)).unwrap();
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
        term.draw(|f| render(f, &mut a)).unwrap();
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
