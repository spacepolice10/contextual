//! Pure selection range math for vim-style visual mode.
//!
//! All positions are `(logical_line, char_col)` with char-based indexing.
//! No ratatui dependency; later tasks (visual state, paint) consume these.
#![allow(dead_code)] // staged helpers until Tasks 2-4 wire consumers

/// Char-wise (`v`) or line-wise (`V`) selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectKind {
    Char,
    Line,
}

/// Anchor end of a visual selection; the other end is the live cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: (usize, usize),
    pub kind: SelectKind,
}

/// Order anchor/cursor into `(start, end)`.
///
/// Pure min/max swap; line-kind full-line expansion is deferred to
/// [`extract_text`] and [`selection_chunks`] (paint task expands there).
pub fn normalize(
    anchor: (usize, usize),
    cursor: (usize, usize),
    _kind: SelectKind,
) -> ((usize, usize), (usize, usize)) {
    if anchor <= cursor {
        (anchor, cursor)
    } else {
        (cursor, anchor)
    }
}

/// Char slice of one logical line; `\r` stripped like `highlight_file`.
fn line_chars(line: &str, from: usize, to: usize) -> String {
    line.replace('\r', "")
        .chars()
        .skip(from)
        .take(to.saturating_sub(from))
        .collect()
}

/// Snippet text for a normalized range (`start <= end`, see [`normalize`]).
///
/// `Char`: substring `[start, end)` (end exclusive, may span lines joined
/// with `\n`). `Line`: full logical lines `start.0..=end.0`. All char-based,
/// out-of-range cols clamp.
pub fn extract_text(
    lines: &[String],
    start: (usize, usize),
    end: (usize, usize),
    kind: SelectKind,
) -> String {
    if lines.is_empty() || start.0 >= lines.len() {
        return String::new();
    }
    // Defensive: callers pass `normalize`d ranges, but never panic on swap.
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let last = end.0.min(lines.len().saturating_sub(1));
    // Clamp a past-EOF end to the end of the last line so the Char
    // single-line check below sees the clamped position.
    let end = if end.0 == last {
        end
    } else {
        (last, usize::MAX)
    };
    match kind {
        SelectKind::Line => (start.0..=last)
            .map(|l| line_chars(&lines[l], 0, usize::MAX))
            .collect::<Vec<_>>()
            .join("\n"),
        SelectKind::Char => {
            if start.0 == end.0 {
                return line_chars(&lines[start.0], start.1, end.1);
            }
            let mut out = line_chars(&lines[start.0], start.1, usize::MAX);
            for line in &lines[start.0 + 1..last] {
                out.push('\n');
                out.push_str(&line_chars(line, 0, usize::MAX));
            }
            out.push('\n');
            out.push_str(&line_chars(&lines[last], 0, end.1));
            out
        }
    }
}

/// Flat char buffer of all lines (`\r` stripped, `\n` separators) plus the
/// flat offset where each logical line starts.
fn flatten(lines: &[String]) -> (Vec<char>, Vec<usize>) {
    let mut flat = Vec::new();
    let mut starts = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            flat.push('\n');
        }
        starts.push(flat.len());
        flat.extend(line.replace('\r', "").chars());
    }
    (flat, starts)
}

/// Map a flat offset back to `(logical_line, char_col)`.
fn unflatten(offset: usize, starts: &[usize]) -> (usize, usize) {
    let line = starts.partition_point(|&s| s <= offset).saturating_sub(1);
    (line, offset - starts[line])
}

/// Vim-style `i`/`a` delimiter object around `cursor`.
///
/// Returns the normalized `(start, end)` range with end exclusive:
/// `inner` (`i`) excludes the delimiters, `!inner` (`a`) includes them.
/// Same-char delimiters (quotes) match the enclosing pair on the cursor
/// line only; distinct delimiters (`(){}[]<>`) nest and may span lines.
/// No escape handling in v1. `None` on miss (caller keeps selection).
pub fn find_delim_pair(
    lines: &[String],
    cursor: (usize, usize),
    open: char,
    close: char,
    inner: bool,
) -> Option<((usize, usize), (usize, usize))> {
    if lines.is_empty() || cursor.0 >= lines.len() {
        return None;
    }
    if open == close {
        let text: Vec<char> = lines[cursor.0].replace('\r', "").chars().collect();
        let col = cursor.1.min(text.len());
        let quotes: Vec<usize> = text
            .iter()
            .enumerate()
            .filter(|(_, &c)| c == open)
            .map(|(i, _)| i)
            .collect();
        // Enclosing consecutive pair; largest opener wins on ties.
        let mut found = None;
        for pair in quotes.windows(2) {
            if pair[0] <= col && col <= pair[1] {
                found = Some((pair[0], pair[1]));
            }
        }
        let (a, b) = found?;
        return Some(if inner {
            ((cursor.0, a + 1), (cursor.0, b))
        } else {
            ((cursor.0, a), (cursor.0, b + 1))
        });
    }
    let (flat, starts) = flatten(lines);
    if flat.is_empty() {
        return None;
    }
    let line_len = lines[cursor.0].replace('\r', "").chars().count();
    let mut coff = starts[cursor.0] + cursor.1.min(line_len);
    coff = coff.min(flat.len().saturating_sub(1));
    // Backward: nearest unmatched opener (cursor on open matches itself,
    // cursor on close skips via depth).
    let mut depth = 0usize;
    let mut open_off = None;
    for i in (0..=coff).rev() {
        if flat[i] == close {
            depth += 1;
        } else if flat[i] == open {
            if depth == 0 {
                open_off = Some(i);
                break;
            }
            depth -= 1;
        }
    }
    let open_off = open_off?;
    // Forward: matching closer with nesting.
    depth = 0;
    let mut close_off = None;
    for (i, &c) in flat.iter().enumerate().skip(open_off) {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                close_off = Some(i);
                break;
            }
        }
    }
    let close_off = close_off?;
    Some(if inner {
        (unflatten(open_off + 1, &starts), unflatten(close_off, &starts))
    } else {
        (
            unflatten(open_off, &starts),
            unflatten(close_off + 1, &starts),
        )
    })
}

/// Intersect a normalized selection `(start, end)` with one visible chunk.
///
/// `line`: logical line being painted; `coff`/`nchars`: visible char window
/// (wrap chunk or `h_scroll` window). Returns chunk-relative ranges (at most
/// one); empty when the selection is not visible here. Mirrors
/// `search::chunk_match_ranges`.
pub fn selection_chunks(
    start: (usize, usize),
    end: (usize, usize),
    line: usize,
    coff: usize,
    nchars: usize,
) -> Vec<(usize, usize)> {
    if nchars == 0 || line < start.0 || line > end.0 {
        return Vec::new();
    }
    let s = if line == start.0 { start.1 } else { 0 };
    let e = if line == end.0 {
        end.1
    } else {
        usize::MAX
    };
    let cs = s.max(coff);
    let ce = e.min(coff + nchars);
    if cs < ce {
        vec![(cs - coff, ce - coff)]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_swaps_anchor_cursor() {
        assert_eq!(
            normalize((3, 5), (1, 2), SelectKind::Char),
            ((1, 2), (3, 5))
        );
    }
    #[test]
    fn extract_multiline_charwise() {
        let lines = vec!["hello".to_string(), "world".to_string()];
        assert_eq!(
            extract_text(&lines, (0, 3), (1, 2), SelectKind::Char),
            "lo\nwo"
        );
    }
    #[test]
    fn delim_inner_quotes_same_line() {
        let lines = vec!["a \"hi\" b".to_string()];
        assert_eq!(
            find_delim_pair(&lines, (0, 4), '"', '"', true),
            Some(((0, 3), (0, 5)))
        );
    }
    #[test]
    fn delim_miss_returns_none() {
        let lines = vec!["no quotes".to_string()];
        assert_eq!(find_delim_pair(&lines, (0, 0), '"', '"', true), None);
    }
    #[test]
    fn chunks_clip_to_window() {
        assert_eq!(
            selection_chunks((0, 1), (0, 5), 0, 2, 3),
            vec![(0, 3)]
        );
    }
    #[test]
    fn extract_clamps_end_past_eof_single_line() {
        let lines = vec!["hi".to_string()];
        assert_eq!(
            extract_text(&lines, (0, 0), (5, 0), SelectKind::Char),
            "hi"
        );
    }
    #[test]
    fn extract_clamps_end_past_eof_last_line() {
        let lines = vec!["aa".to_string(), "bb".to_string()];
        assert_eq!(
            extract_text(&lines, (1, 0), (5, 0), SelectKind::Char),
            "bb"
        );
    }
    #[test]
    fn extract_clamps_end_past_eof_multiline() {
        let lines = vec!["aa".to_string(), "bb".to_string()];
        assert_eq!(
            extract_text(&lines, (0, 1), (5, 0), SelectKind::Char),
            "a\nbb"
        );
    }
}
