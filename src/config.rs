use crate::highlight::Theme;
use std::path::PathBuf;

/// Test-only override for the config path: mutating process env in a
/// multithreaded test binary races libc `getenv`/`setenv` (observed as
/// `load_theme` flap under `--test-threads=32`), so tests point here
/// instead of swapping `$XDG_CONFIG_HOME`. Production always uses env.
#[cfg(test)]
static TEST_CONFIG_FILE: std::sync::Mutex<Option<PathBuf>> =
    std::sync::Mutex::new(None);

/// Config file holding the saved theme id (one line, e.g. `dracula`).
/// Honors `$XDG_CONFIG_HOME`, else `~/.config`.
pub fn config_file() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(p) = TEST_CONFIG_FILE.lock().unwrap().clone() {
        return Some(p);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("contextual/theme"))
}

/// Saved theme, if present and recognized. Missing/corrupt file -> None
/// (caller falls back to terminal auto-detect). Never fails.
pub fn load_theme() -> Option<Theme> {
    let text = std::fs::read_to_string(config_file()?).ok()?;
    Theme::from_name(&text)
}

/// Persist the theme choice; creates parent dirs. Best-effort: I/O
/// errors are ignored so a bad home dir can never break the viewer.
pub fn save_theme(theme: Theme) {
    let Some(path) = config_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, format!("{}\n", theme.name()));
}

/// Serialize tests that point the config elsewhere (the override is
/// process-global; readers only take the inner mutex briefly, which is
/// sound unlike libc `setenv` races).
#[cfg(test)]
pub(crate) fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
fn test_override() -> std::sync::MutexGuard<'static, Option<PathBuf>> {
    TEST_CONFIG_FILE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Point the config path at `path` for tests (no env mutation, no race).
#[cfg(test)]
pub(crate) fn set_test_config(path: PathBuf) {
    *test_override() = Some(path);
}

/// Drop the test override, restoring env-based lookup.
#[cfg(test)]
pub(crate) fn clear_test_config() {
    *test_override() = None;
}

/// RAII isolated config: unique temp file as the config path, override
/// cleared and dir removed on drop (even on panic, so no test pollutes
/// the next). Hold the return value for the whole test.
#[cfg(test)]
pub(crate) struct TestConfigGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    dir: PathBuf,
}

/// Isolate the config path for one test (see [`TestConfigGuard`]).
#[cfg(test)]
pub(crate) fn isolated_test_config(tag: &str) -> TestConfigGuard {
    let lock = test_lock();
    let dir = std::env::temp_dir().join(format!(
        "ctx_testcfg_{}_{}_{}",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    set_test_config(dir.join("theme"));
    TestConfigGuard { _lock: lock, dir }
}

#[cfg(test)]
impl Drop for TestConfigGuard {
    fn drop(&mut self) {
        clear_test_config();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{isolated_test_config, load_theme, save_theme};
    use crate::highlight::Theme;

    #[test]
    fn saved_theme_round_trips_and_bad_content_is_none() {
        let _cfg = isolated_test_config("roundtrip");
        assert_eq!(load_theme(), None);
        save_theme(Theme::Dracula);
        assert_eq!(load_theme(), Some(Theme::Dracula));
        std::fs::write(
            crate::config::config_file().unwrap(),
            "nope\n",
        )
        .unwrap();
        assert_eq!(load_theme(), None);
    }
}
