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
    }
}
