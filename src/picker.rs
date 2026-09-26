use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
}

/// List regular files in `dir`, sorted by name. Skips dirs/symlink-dirs/errors.
pub fn list_files(dir: &Path) -> Result<Vec<FileEntry>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(FileEntry {
            name,
            path,
            size: meta.len(),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Cap on picker entries; `discover_files` stops the walk here and reports
/// `truncated = true` so the UI can show `[truncated at 50k]`.
/// Staged helper (wired in Task 6); allow dead code until then.
#[allow(dead_code)]
pub const MAX_PICKER_FILES: usize = 50_000;

/// Recursively list regular files under `root`, sorted by name.
/// Shows hidden files but respects `.gitignore`/`.ignore`/excludes, and works
/// outside git repos. Skips dirs, symlink-dirs, and errors; never aborts.
/// Returns `(entries, truncated)`.
/// Staged helper (wired in Task 6); allow dead code until then.
#[allow(dead_code)]
pub fn discover_files(root: &Path) -> (Vec<FileEntry>, bool) {
    let mut out = Vec::new();
    let mut truncated = false;
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .parents(true)
        .require_git(false)
        .follow_links(false)
        .build();
    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(FileEntry {
            name,
            path,
            size: meta.len(),
        });
        if out.len() >= MAX_PICKER_FILES {
            truncated = true;
            break;
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    (out, truncated)
}

/// Clamp selection index into 0..len.
pub fn clamp_selection(idx: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    idx.min(len - 1)
}

// src/picker.rs tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    fn unique_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ctx_picker_{}_{}_{}",
            std::process::id(),
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
    #[test]
    fn discover_respects_gitignore_but_shows_hidden() {
        let dir = unique_dir("gi");
        fs::write(dir.join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(dir.join("kept.txt"), "k").unwrap();
        fs::write(dir.join(".hidden"), "h").unwrap();
        fs::write(dir.join("ignored.txt"), "x").unwrap();
        let (files, truncated) = discover_files(&dir);
        let mut names: Vec<_> = files.iter().map(|f| f.name.clone()).collect();
        names.sort();
        // `.gitignore` itself is a real (hidden, shown) file; only its target is excluded.
        assert_eq!(
            names,
            vec![
                ".gitignore".to_string(),
                ".hidden".to_string(),
                "kept.txt".to_string()
            ]
        );
        assert!(!truncated);
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn discover_recursive_and_caps_truncation_flag() {
        let dir = unique_dir("rec");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub").join("deep.txt"), "d").unwrap();
        fs::write(dir.join("top.txt"), "t").unwrap();
        let (files, truncated) = discover_files(&dir);
        assert!(files.iter().any(|f| f.name == "deep.txt"));
        assert!(!truncated);
        assert!(files.len() <= MAX_PICKER_FILES);
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn discover_skips_dirs_and_errors() {
        let dir = unique_dir("dirs");
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("a.txt"), "a").unwrap();
        let (files, _) = discover_files(&dir);
        assert!(files.iter().all(|f| f.name != "subdir"));
        assert!(files.iter().any(|f| f.name == "a.txt"));
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    #[cfg(unix)]
    fn discover_skips_symlink_loop() {
        let dir = unique_dir("link");
        fs::write(dir.join("a.txt"), "a").unwrap();
        std::os::unix::fs::symlink(&dir, dir.join("loop")).unwrap();
        let (files, _) = discover_files(&dir);
        assert!(files.iter().any(|f| f.name == "a.txt"));
        assert!(files.iter().all(|f| f.name != "loop"));
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn clamp_selection_empty_is_zero() {
        assert_eq!(clamp_selection(5, 0), 0);
    }
    #[test]
    fn clamp_selection_bounds() {
        assert_eq!(clamp_selection(0, 3), 0);
        assert_eq!(clamp_selection(99, 3), 2);
        assert_eq!(clamp_selection(1, 3), 1);
    }
    #[test]
    fn list_files_only_regular_sorted() {
        let dir = std::env::temp_dir().join(format!("ctx_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("b.txt"), "b").unwrap();
        fs::write(dir.join("a.txt"), "a").unwrap();
        let files = list_files(&dir).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.name.clone()).collect();
        assert_eq!(names, vec!["a.txt".to_string(), "b.txt".to_string()]);
        let _ = fs::remove_dir_all(&dir);
    }
}
