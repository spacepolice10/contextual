pub mod picker;
pub mod viewer;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};

/// Max accumulated count prefix (`v3w`, `5j`); mirrors `search::MAX_MATCHES`
/// style so huge digit runs can't hang repeat loops or wrap `as isize`.
pub(crate) const MAX_COUNT: usize = 10_000;

pub fn handle(app: &mut crate::app::App, code: KeyCode, mods: KeyModifiers) -> Result<bool> {
    use crate::app::Mode;
    if mods.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        return Ok(true);
    }
    match app.mode {
        Mode::Picker => picker::handle_picker(app, code, mods),
        Mode::Viewer => viewer::handle_viewer(app, code, mods),
        Mode::Plan => {
            if let Some(pm) = &mut app.plan {
                match code {
                    KeyCode::Up | KeyCode::Char('k') => pm.up(),
                    KeyCode::Down | KeyCode::Char('j') => pm.down(),
                    KeyCode::Left | KeyCode::Char('h') => pm.parent(),
                    KeyCode::Right | KeyCode::Char('l') => pm.child(),
                    KeyCode::PageUp => pm.page_up(),
                    KeyCode::PageDown => pm.page_down(),
                    _ => {}
                }
            }
            Ok(false)
        }
    }
}
