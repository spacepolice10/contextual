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
