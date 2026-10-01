use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) fn handle_picker(
    app: &mut crate::app::App,
    code: KeyCode,
    mods: KeyModifiers,
) -> Result<bool> {
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
                KeyCode::Enter => app.open_selected_entry(),
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
#[cfg(test)]
mod tests {
    use crate::input::handle;
    use crossterm::event::{KeyCode, KeyModifiers};
    fn picker_entry(path: &str) -> crate::file_picker::FileEntry {
        crate::file_picker::FileEntry {
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            path: std::path::PathBuf::from(path),
            size: 0,
        }
    }
    fn picker_app() -> crate::app::App {
        crate::app::App::new_picker(vec![picker_entry("b.txt"), picker_entry("a.txt")], false)
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
        let mut a = crate::app::App::new_picker(
            vec![
                crate::file_picker::FileEntry {
                    name: "first.txt".to_string(),
                    path: first,
                    size: 6,
                },
                crate::file_picker::FileEntry {
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
        assert_eq!(a.mode, crate::app::Mode::Viewer);
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
        assert_eq!(a.mode, crate::app::Mode::Picker);
    }
}
