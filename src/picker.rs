use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    /// Byte size (populate at discovery; render omits it in v1 — Task 6 decides).
    #[allow(dead_code)]
    pub size: u64,
}

/// Cap on picker entries; `discover_files` stops the walk here and reports
/// `truncated = true` so the UI can show `[truncated at 50k]`.
pub const MAX_PICKER_FILES: usize = 50_000;

/// Recursively list regular files under `root`, sorted by name.
/// Shows hidden files but respects `.gitignore`/`.ignore`/excludes, and works
/// outside git repos. Skips dirs, symlink-dirs, and errors; never aborts.
/// Returns `(entries, truncated)`.
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

/// One fuzzy hit: index into the entry slice, nucleo score, and matched
/// columns as char indices into [`display_path`] (for highlight painting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoredMatch {
    pub entry_idx: usize,
    pub score: u32,
    pub cols: Vec<usize>,
}

/// Display string for ranking and painting: lossy path with `/` separators
/// (so Windows paths rank like posix ones).
pub fn display_path(entry: &FileEntry) -> String {
    entry.path.to_string_lossy().replace('\\', "/")
}

/// Fuzzy-filter `entries` by `query` with nucleo (smart-case: any uppercase
/// makes the match case-sensitive). Empty query returns every entry
/// name-sorted with no columns. Results sort by score desc, name for ties.
pub fn filter_files(entries: &[FileEntry], query: &str) -> Vec<ScoredMatch> {
    if query.is_empty() {
        let mut idx: Vec<usize> = (0..entries.len()).collect();
        idx.sort_by(|&a, &b| entries[a].name.cmp(&entries[b].name));
        return idx
            .into_iter()
            .map(|entry_idx| ScoredMatch {
                entry_idx,
                score: 0,
                cols: Vec::new(),
            })
            .collect();
    }
    use nucleo_matcher::{
        pattern::{AtomKind, CaseMatching, Normalization, Pattern},
        Config, Matcher, Utf32Str,
    };
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let pattern = Pattern::new(
        query,
        CaseMatching::Smart,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );
    let mut out = Vec::new();
    let mut buf = Vec::new();
    let mut indices = Vec::new();
    for (entry_idx, entry) in entries.iter().enumerate() {
        let shown = display_path(entry);
        buf.clear();
        let haystack = Utf32Str::new(&shown, &mut buf);
        indices.clear();
        if let Some(score) = pattern.indices(haystack, &mut matcher, &mut indices) {
            indices.sort_unstable();
            indices.dedup();
            out.push(ScoredMatch {
                entry_idx,
                score,
                cols: indices.iter().map(|&i| i as usize).collect(),
            });
        }
    }
    out.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| entries[a.entry_idx].name.cmp(&entries[b.entry_idx].name))
    });
    out
}

/// Picker prompt text: `> query  n/m` with `[no matches]` / `[truncated…]`
/// notes. The block cursor cell is appended by `render`.
pub fn picker_prompt_line(query: &str, filtered: usize, total: usize, truncated: bool) -> String {
    let mut s = format!("> {query}  {filtered}/{total}");
    if filtered == 0 {
        s.push_str("  [no matches]");
    }
    if truncated {
        s.push_str("  [truncated at 50k]");
    }
    s
}

/// Preview pane visibility: hidden on narrow terminals (results full-width).
pub fn picker_preview_visible(width: usize) -> bool {
    width >= 80
}

/// Max preview bytes (alongside the `max_lines` cap at call sites).
pub const PREVIEW_MAX_BYTES: usize = 100 * 1024;

/// Capped lossy preview read: at most `PREVIEW_MAX_BYTES` off disk, then
/// `max_lines` lines. Returns lines plus a `[binary preview]` note when NUL
/// bytes are present. Never fails: unreadable files yield empty lines.
/// The byte cap binds at read time (not after) so selecting a huge file
/// cannot stall the render loop.
pub fn read_preview_lines(path: &Path, max_lines: usize) -> (Vec<String>, Option<String>) {
    use std::io::Read;
    let mut buf = Vec::new();
    if let Ok(f) = std::fs::File::open(path) {
        let _ = f.take(PREVIEW_MAX_BYTES as u64).read_to_end(&mut buf);
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let binary = buf.contains(&0);
    let lines: Vec<String> = text.lines().take(max_lines).map(|s| s.to_string()).collect();
    let note = binary.then(|| "[binary preview]".to_string());
    (lines, note)
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
    fn entry(path: &str) -> FileEntry {
        FileEntry {
            name: path.rsplit('/').next().unwrap_or(path).to_string(),
            path: PathBuf::from(path),
            size: 0,
        }
    }
    #[test]
    fn empty_query_returns_name_sorted_without_cols() {
        let entries = vec![entry("b.txt"), entry("a.txt")];
        let got = filter_files(&entries, "");
        assert_eq!(got.len(), 2);
        // Name-sorted: a.txt (idx 1) before b.txt (idx 0).
        assert_eq!(got[0].entry_idx, 1);
        assert_eq!(got[1].entry_idx, 0);
        assert!(got.iter().all(|m| m.cols.is_empty()));
    }
    #[test]
    fn fuzzy_prefers_filename_and_reports_char_cols() {
        let entries = vec![entry("docs/notes.md"), entry("src/main.rs")];
        let got = filter_files(&entries, "main");
        assert!(!got.is_empty());
        assert_eq!(got[0].entry_idx, 1);
        assert!(!got[0].cols.is_empty());
        // Cols are valid char indices whose chars spell the query.
        let shown = display_path(&entries[got[0].entry_idx]);
        let chars: Vec<char> = shown.chars().collect();
        for &c in &got[0].cols {
            assert!(c < chars.len());
        }
        let hit: String = got[0].cols.iter().map(|&c| chars[c]).collect();
        assert_eq!(hit.to_lowercase(), "main");
        // Non-contiguous fuzzy still matches across separators.
        let fuzzy = filter_files(&entries, "smrs");
        assert!(fuzzy.iter().any(|m| m.entry_idx == 1));
    }
    #[test]
    fn unicode_cols_are_char_not_byte() {
        let entries = vec![entry("héllo_🌍.txt")];
        let got = filter_files(&entries, "🌍");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].cols, vec![6]);
    }
    #[test]
    fn smart_case_upper_is_sensitive() {
        let entries = vec![entry("src/main.rs")];
        assert!(filter_files(&entries, "Main").is_empty());
        assert_eq!(filter_files(&entries, "main").len(), 1);
    }
    #[test]
    fn discover_flat_names_stay_sorted() {
        // `discover_files` superseded the flat lister; name order is pinned here.
        let dir = unique_dir("flat");
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("b.txt"), "b").unwrap();
        fs::write(dir.join("a.txt"), "a").unwrap();
        let (files, _) = discover_files(&dir);
        let names: Vec<_> = files.iter().map(|f| f.name.clone()).collect();
        assert_eq!(names, vec!["a.txt".to_string(), "b.txt".to_string()]);
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn picker_prompt_shows_count() {
        let s = picker_prompt_line("mr", 3, 120, false);
        assert!(s.contains("> mr"), "unexpected: {s}");
        assert!(s.contains("3/120"), "unexpected: {s}");
    }
    #[test]
    fn picker_prompt_flags_empty_and_truncated() {
        let s = picker_prompt_line("zzz", 0, 40, false);
        assert!(s.contains("[no matches]"), "unexpected: {s}");
        let s = picker_prompt_line("", 50_000, 50_000, true);
        assert!(s.contains("[truncated at 50k]"), "unexpected: {s}");
    }
    #[test]
    fn picker_preview_cap_truncates() {
        let dir = unique_dir("prev");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.txt");
        let body: String = (0..1000).map(|i| format!("line {i}\n")).collect();
        std::fs::write(&path, body).unwrap();
        let (lines, note) = read_preview_lines(&path, 200);
        assert_eq!(lines.len(), 200);
        assert_eq!(lines[0], "line 0");
        assert!(note.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn picker_preview_read_is_byte_capped() {
        // 300KB file with a huge line cap: only the 100KB byte cap may bind.
        let dir = unique_dir("prevcap");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("wide.bin");
        let chunk = "x".repeat(1023) + "\n";
        let body = chunk.repeat(300);
        assert!(body.len() >= 300 * 1024);
        std::fs::write(&path, body.as_bytes()).unwrap();
        let (lines, _) = read_preview_lines(&path, usize::MAX);
        let total: usize = lines.iter().map(|l| l.len()).sum();
        assert!(
            total <= 100 * 1024,
            "preview read {total} bytes without a bound"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn picker_preview_binary_shows_note() {
        let dir = unique_dir("prevbin");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bin.dat");
        std::fs::write(&path, b"ab\x00cd\nline2\n").unwrap();
        let (lines, note) = read_preview_lines(&path, 200);
        assert!(!lines.is_empty());
        assert!(lines.len() <= 200);
        assert_eq!(note.as_deref(), Some("[binary preview]"));
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn picker_preview_narrow_hidden() {
        assert!(!picker_preview_visible(79));
        assert!(picker_preview_visible(80));
        assert!(!picker_preview_visible(0));
    }
    #[test]
    fn truncated_flag_reaches_prompt() {
        let a = crate::app::App::new_picker(
            vec![FileEntry {
                name: "a.txt".to_string(),
                path: std::path::PathBuf::from("a.txt"),
                size: 1,
            }],
            true,
        );
        assert!(a.picker.truncated);
        let s = picker_prompt_line(
            &a.picker.query,
            a.picker.filtered.len(),
            a.files.len(),
            a.picker.truncated,
        );
        assert!(s.contains("50k"), "unexpected: {s}");
    }
}
