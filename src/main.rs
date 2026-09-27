mod app;
mod cli;
mod highlight;
mod input;
mod picker;
mod search;
mod select;
mod tui;
mod ui;
mod viewer;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event};
use ratatui::{backend::CrosstermBackend, Terminal};
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
                if input::handle(&mut app, k.code, k.modifiers)? {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn render(f: &mut ratatui::Frame, app: &mut app::App) {
    use app::Mode;
    let area = f.area();
    match app.mode {
        Mode::Picker => crate::ui::picker::render_picker(f, app, area),
        Mode::Viewer => crate::ui::viewer::render_viewer(f, app, area),
    }
}



#[cfg(test)]
mod status_tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    fn viewer() -> app::App {
        app::App::load_file(std::path::Path::new("Cargo.toml"), false).unwrap()
    }
    #[test]
    fn status_shows_visual_and_comment_count() {
        use crate::ui::status::viewer_hint;
        let mut a = viewer();
        a.lines = vec!["hi".to_string()];
        crate::input::handle(&mut a, KeyCode::Char('v'), KeyModifiers::NONE).unwrap();
        assert!(viewer_hint(false).contains("C"));
    }
}
