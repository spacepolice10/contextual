mod app;
mod highlight;
mod picker;
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
                app.viewport_h = term.size()?.height.saturating_sub(3) as usize;
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
        Mode::Viewer => match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if app.from_picker && code == KeyCode::Esc {
                    let files = picker::list_files(std::path::Path::new("."))?;
                    *app = app::App::new_picker(files);
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.move_cursor_line(1);
                Ok(false)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.move_cursor_line(-1);
                Ok(false)
            }
            KeyCode::PageDown => {
                let vh = app.viewport_h;
                app.move_cursor_line(vh as isize);
                Ok(false)
            }
            KeyCode::PageUp => {
                let vh = app.viewport_h;
                app.move_cursor_line(-(vh as isize));
                Ok(false)
            }
            KeyCode::Home | KeyCode::Char('g') => {
                app.cursor_line = 0;
                app.cursor_col = 0;
                app.scroll = 0;
                Ok(false)
            }
            KeyCode::End | KeyCode::Char('G') => {
                if !app.lines.is_empty() {
                    app.cursor_line = app.lines.len() - 1;
                    app.cursor_col = app.lines[app.cursor_line].chars().count();
                }
                app.scroll = usize::MAX;
                Ok(false)
            }
            KeyCode::Char('w') => {
                app.toggle_wrap();
                Ok(false)
            }
            KeyCode::Left | KeyCode::Char('h') => {
                app.move_cursor_col(-1);
                Ok(false)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                app.move_cursor_col(1);
                Ok(false)
            }
            _ => Ok(false),
        },
    }
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
                        .title(" contextual — pick a file (Enter/q) "),
                )
                .highlight_style(Style::default().bg(Color::DarkGray));
            if app.files.is_empty() {
                let p = Paragraph::new("No files in this directory — press q to quit.")
                    .block(Block::default().borders(Borders::ALL).title(" contextual "));
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
            let text_w = (chunks[0].width as usize).saturating_sub(gutter + 1);
            let display = viewer::build_display_lines(&app.lines, text_w.max(1), app.wrap);
            let total = display.len();
            let vh = chunks[0].height as usize;
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
                let windowed: Vec<Span> = match src {
                    Some(spans) if app.wrap => highlight::slice_spans(spans, chunk_k * w, w),
                    Some(spans) => highlight::slice_spans(spans, app.h_scroll, w),
                    None => vec![Span::raw(shown.clone())],
                };
                let hl_row = src.is_some();
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
                            s.style = cursor_style;
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
                    .title(format!(" {} ", app.filename)),
            );
            f.render_widget(body, chunks[0]);
            let mut s = format!(
                " {}  {}/{}  Ln {},Col {}  wrap:{}  [w]rap [q]uit ",
                app.filename,
                app.scroll + 1,
                total.max(1),
                app.cursor_line + 1,
                app.cursor_col + 1,
                if app.wrap { "ON" } else { "OFF" }
            );
            if let Some(n) = &app.status_note {
                s.push_str(n);
            }
            if app.lang.is_some() && app.highlighted.is_none() {
                s.push_str("[no highlight]");
            }
            let bar = Paragraph::new(vec![
                Line::from(Span::raw(s)),
                Line::from(Span::styled(
                    " hjkl/arrows move cursor  PgUp/PgDn  g/G top/bottom  w wrap  q quit ",
                    Style::default().fg(Color::Gray),
                )),
            ]);
            f.render_widget(bar, chunks[1]);
        }
    }
}
