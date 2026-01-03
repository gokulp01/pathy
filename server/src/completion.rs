use std::path::{Path, PathBuf};

use lsp_types::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringContext {
    pub content_before_cursor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathQuery {
    pub dir_part: String,
    pub segment_prefix: String,
    pub path_str: String,
}

pub fn find_string_context(line: &str, cursor_byte: usize) -> Option<StringContext> {
    let mut in_quote: Option<char> = None;
    let mut quote_start = 0usize;
    let mut i = 0usize;
    while i < line.len() && i < cursor_byte {
        let ch = line[i..].chars().next()?;
        let ch_len = ch.len_utf8();
        if in_quote.is_none() && ch == '#' {
            return None;
        }
        if ch == '\\' {
            i += ch_len;
            if i < cursor_byte {
                let next_ch = line[i..].chars().next()?;
                i += next_ch.len_utf8();
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            if let Some(current) = in_quote {
                if current == ch {
                    in_quote = None;
                }
            } else {
                in_quote = Some(ch);
                quote_start = i;
            }
        }
        i += ch_len;
    }

    if in_quote.is_none() {
        return None;
    }
    if cursor_byte < quote_start + 1 || cursor_byte > line.len() {
        return None;
    }
    Some(StringContext {
        content_before_cursor: line[quote_start + 1..cursor_byte].to_string(),
    })
}

fn is_boundary_char(ch: char) -> bool {
    ch.is_whitespace()
}

fn is_windows_drive_prefix(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(letter) = chars.next() else {
        return false;
    };
    let Some(colon) = chars.next() else {
        return false;
    };
    if !letter.is_ascii_alphabetic() || colon != ':' {
        return false;
    }
    matches!(chars.next(), Some('\\') | Some('/'))
}

pub fn find_path_query(content_before_cursor: &str) -> Option<PathQuery> {
    if content_before_cursor.is_empty() {
        return None;
    }

    let mut last_start: Option<usize> = None;
    let mut prev_char: Option<char> = None;
    for (idx, ch) in content_before_cursor.char_indices() {
        if let Some(prev) = prev_char {
            if !is_boundary_char(prev) {
                prev_char = Some(ch);
                continue;
            }
        }

        let remainder = &content_before_cursor[idx..];
        if remainder.starts_with("../")
            || remainder.starts_with("./")
            || remainder.starts_with("/")
            || remainder.starts_with("~")
            || remainder.starts_with("\\\\")
            || is_windows_drive_prefix(remainder)
        {
            last_start = Some(idx);
        }

        prev_char = Some(ch);
    }

    let start = last_start?;
    let path_str = &content_before_cursor[start..];
    let mut last_sep = None;
    for (idx, ch) in path_str.char_indices() {
        if ch == '/' || ch == '\\' {
            last_sep = Some(idx);
        }
    }

    let (dir_part, segment_prefix) = if let Some(sep_idx) = last_sep {
        (
            path_str[..sep_idx + 1].to_string(),
            path_str[sep_idx + 1..].to_string(),
        )
    } else {
        ("".to_string(), path_str.to_string())
    };

    if path_str == "~" {
        return Some(PathQuery {
            dir_part: "~/".to_string(),
            segment_prefix: "".to_string(),
            path_str: path_str.to_string(),
        });
    }

    Some(PathQuery {
        dir_part,
        segment_prefix,
        path_str: path_str.to_string(),
    })
}

pub fn base_dir_from_uri(uri: &Url, root_uri: Option<&Url>) -> Option<PathBuf> {
    if uri.scheme() == "file" {
        if let Ok(path) = uri.to_file_path() {
            return path.parent().map(|p| p.to_path_buf());
        }
    }
    root_uri.and_then(|root| root.to_file_path().ok())
}

pub fn resolve_list_dir(query: &PathQuery, base_dir: Option<&Path>) -> Option<PathBuf> {
    let path_str = query.path_str.as_str();

    if path_str.starts_with("~") {
        let home = dirs_home()?;
        let remainder = query.dir_part.trim_start_matches('~');
        return Some(home.join(remainder.trim_start_matches('/')));
    }

    if path_str.starts_with('/') {
        return Some(PathBuf::from(&query.dir_part));
    }

    #[cfg(windows)]
    {
        if path_str.starts_with("\\\\") || is_windows_drive_prefix(path_str) {
            return Some(PathBuf::from(&query.dir_part));
        }
    }

    let base = base_dir?;
    if query.dir_part.is_empty() {
        return Some(base.to_path_buf());
    }
    Some(base.join(&query.dir_part))
}

fn dirs_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("HOME") {
        return Some(PathBuf::from(home));
    }
    if let Some(home) = std::env::var_os("USERPROFILE") {
        return Some(PathBuf::from(home));
    }
    None
}

pub fn filter_and_sort_entries(
    entries: Vec<(String, bool)>,
    segment_prefix: &str,
) -> Vec<(String, bool)> {
    let mut filtered: Vec<(String, bool)> = entries
        .into_iter()
        .filter(|(name, _)| name.starts_with(segment_prefix))
        .collect();
    filtered.sort_by(|(a_name, a_dir), (b_name, b_dir)| {
        b_dir.cmp(a_dir).then_with(|| a_name.cmp(b_name))
    });
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_string_context_simple() {
        let line = "open(\"./foo\")";
        let cursor = line.find("./foo").unwrap() + "./foo".len();
        let ctx = find_string_context(line, cursor).unwrap();
        assert_eq!(ctx.content_before_cursor, "./foo");
    }

    #[test]
    fn detects_path_query_with_segment() {
        let query = find_path_query("./dir/pa").unwrap();
        assert_eq!(query.dir_part, "./dir/");
        assert_eq!(query.segment_prefix, "pa");
    }

    #[test]
    fn detects_path_query_home() {
        let query = find_path_query("~/Do").unwrap();
        assert_eq!(query.dir_part, "~/");
        assert_eq!(query.segment_prefix, "Do");
    }

    #[test]
    fn base_dir_from_file_uri() {
        let uri = Url::parse("file:///tmp/example.py").unwrap();
        let base = base_dir_from_uri(&uri, None).unwrap();
        assert!(base.ends_with("/tmp"));
    }

    #[test]
    fn filter_and_sort_dirs_first() {
        let entries = vec![
            ("b.txt".to_string(), false),
            ("a".to_string(), true),
            ("a.txt".to_string(), false),
        ];
        let filtered = filter_and_sort_entries(entries, "");
        assert_eq!(filtered[0].0, "a");
        assert!(filtered[0].1);
    }
}
