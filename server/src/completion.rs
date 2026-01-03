use std::path::{Path, PathBuf};

use lsp_types::Url;

use crate::config::{BaseDirStrategy, Config, WorkspaceRootStrategy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixKind {
    Relative,
    Absolute,
    Home,
    WindowsDrive,
    WindowsUnc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathQuery {
    pub dir_part: String,
    pub segment_prefix: String,
    pub path_str: String,
    pub prefix_kind: PrefixKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringInfo {
    pub content_before_cursor: String,
    pub is_raw: bool,
    pub is_fstring: bool,
    pub string_start_byte: usize,
    pub string_start_utf16: u32,
}

pub fn find_string_info(line: &str, cursor_byte: usize) -> Option<StringInfo> {
    let mut in_string: Option<StringState> = None;
    let mut i = 0usize;
    while i < line.len() {
        if i == cursor_byte {
            if let Some(state) = &in_string {
                let content_start = state.start_byte + state.delim_len;
                if cursor_byte >= content_start {
                    let content = &line[content_start..cursor_byte];
                    if state.is_fstring && is_in_interpolation(content, state.is_raw) {
                        return None;
                    }
                    let string_start_utf16 = utf16_len(&line[..content_start]);
                    return Some(StringInfo {
                        content_before_cursor: content.to_string(),
                        is_raw: state.is_raw,
                        is_fstring: state.is_fstring,
                        string_start_byte: state.start_byte,
                        string_start_utf16,
                    });
                }
            }
        }

        let ch = line[i..].chars().next()?;
        let ch_len = ch.len_utf8();
        if in_string.is_none() && ch == '#' {
            break;
        }

        if let Some(state) = &mut in_string {
            if !state.is_raw && ch == '\\' {
                i += ch_len;
                if i < line.len() {
                    let next = line[i..].chars().next()?;
                    i += next.len_utf8();
                    continue;
                }
            }
            if state.delim_len == 1 && ch == state.quote {
                in_string = None;
            } else if state.delim_len == 3 && line[i..].starts_with(state.delim.as_str()) {
                i += state.delim_len;
                in_string = None;
                continue;
            }
            i += ch_len;
            continue;
        }

        if ch == '\'' || ch == '"' {
            let (is_raw, is_fstring) = detect_prefix(line, i);
            let delim_len = if line[i..].starts_with("\"\"\"") || line[i..].starts_with("'''") {
                3
            } else {
                1
            };
            let delim = if delim_len == 3 {
                if ch == '\'' {
                    "'''"
                } else {
                    "\"\"\""
                }
            } else {
                ""
            };
            in_string = Some(StringState {
                quote: ch,
                delim_len,
                delim: delim.to_string(),
                start_byte: i,
                is_raw,
                is_fstring,
            });
            i += ch_len;
            continue;
        }

        i += ch_len;
    }

    if cursor_byte == line.len() {
        if let Some(state) = in_string {
            let content_start = state.start_byte + state.delim_len;
            if cursor_byte >= content_start {
                let content = &line[content_start..cursor_byte];
                if state.is_fstring && is_in_interpolation(content, state.is_raw) {
                    return None;
                }
                let string_start_utf16 = utf16_len(&line[..content_start]);
                return Some(StringInfo {
                    content_before_cursor: content.to_string(),
                    is_raw: state.is_raw,
                    is_fstring: state.is_fstring,
                    string_start_byte: state.start_byte,
                    string_start_utf16,
                });
            }
        }
    }

    None
}

#[derive(Debug, Clone)]
struct StringState {
    quote: char,
    delim_len: usize,
    delim: String,
    start_byte: usize,
    is_raw: bool,
    is_fstring: bool,
}

fn detect_prefix(line: &str, quote_idx: usize) -> (bool, bool) {
    let mut prefix = String::new();
    let mut i = quote_idx;
    while i > 0 {
        let prev = line[..i].chars().last().unwrap_or(' ');
        if !prev.is_ascii_alphabetic() {
            break;
        }
        prefix.push(prev.to_ascii_lowercase());
        i -= prev.len_utf8();
        if prefix.len() >= 2 {
            break;
        }
    }

    if i > 0 {
        let before = line[..i].chars().last().unwrap_or(' ');
        if before.is_ascii_alphanumeric() || before == '_' {
            return (false, false);
        }
    }

    let is_raw = prefix.contains('r');
    let is_fstring = prefix.contains('f');
    (is_raw, is_fstring)
}

fn is_in_interpolation(content: &str, is_raw: bool) -> bool {
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < content.len() {
        let ch = content[i..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        if !is_raw && ch == '\\' {
            i += ch_len;
            if i < content.len() {
                let next = content[i..].chars().next().unwrap();
                i += next.len_utf8();
            }
            continue;
        }
        if ch == '{' {
            if content[i + ch_len..].starts_with('{') {
                i += ch_len * 2;
                continue;
            }
            depth += 1;
        } else if ch == '}' {
            if content[i + ch_len..].starts_with('}') {
                i += ch_len * 2;
                continue;
            }
            if depth > 0 {
                depth -= 1;
            }
        }
        i += ch_len;
    }
    depth > 0
}

pub fn find_prefix_query(content_before_cursor: &str, config: &Config) -> Option<PathQuery> {
    if content_before_cursor.is_empty() {
        return None;
    }

    let mut last_start: Option<usize> = None;
    let mut prev_char: Option<char> = None;
    for (idx, ch) in content_before_cursor.char_indices() {
        if let Some(prev) = prev_char {
            if !prev.is_whitespace() {
                prev_char = Some(ch);
                continue;
            }
        }

        let remainder = &content_before_cursor[idx..];
        if remainder.starts_with("../")
            || remainder.starts_with("./")
            || remainder.starts_with("/")
            || remainder.starts_with("~")
            || (config.windows_enable_unc && remainder.starts_with("\\\\"))
            || (config.windows_enable_drive_prefix && is_windows_drive_prefix(remainder))
        {
            last_start = Some(idx);
        }

        prev_char = Some(ch);
    }

    let start = last_start?;
    let path_str = &content_before_cursor[start..];
    let prefix_kind = prefix_kind_for_path(path_str, config);
    let (dir_part, segment_prefix) = split_dir_and_segment(path_str);

    Some(PathQuery {
        dir_part,
        segment_prefix,
        path_str: path_str.to_string(),
        prefix_kind,
    })
}

pub fn build_relative_query(content_before_cursor: &str) -> PathQuery {
    let (dir_part, segment_prefix) = split_dir_and_segment(content_before_cursor);
    PathQuery {
        dir_part,
        segment_prefix,
        path_str: content_before_cursor.to_string(),
        prefix_kind: PrefixKind::Relative,
    }
}

fn split_dir_and_segment(path_str: &str) -> (String, String) {
    let mut last_sep = None;
    for (idx, ch) in path_str.char_indices() {
        if ch == '/' || ch == '\\' {
            last_sep = Some(idx);
        }
    }

    if let Some(sep_idx) = last_sep {
        (
            path_str[..sep_idx + 1].to_string(),
            path_str[sep_idx + 1..].to_string(),
        )
    } else {
        ("".to_string(), path_str.to_string())
    }
}

pub fn prefix_kind_for_path(path_str: &str, config: &Config) -> PrefixKind {
    if path_str.starts_with('~') {
        return PrefixKind::Home;
    }
    if path_str.starts_with('/') {
        return PrefixKind::Absolute;
    }
    if config.windows_enable_unc && path_str.starts_with("\\\\") {
        return PrefixKind::WindowsUnc;
    }
    if config.windows_enable_drive_prefix && is_windows_drive_prefix(path_str) {
        return PrefixKind::WindowsDrive;
    }
    PrefixKind::Relative
}

pub fn base_dir_from_uri(uri: &Url, root_uri: Option<&Url>) -> Option<PathBuf> {
    if uri.scheme() == "file" {
        if let Ok(path) = uri.to_file_path() {
            return path.parent().map(|p| p.to_path_buf());
        }
    }
    root_uri.and_then(|root| root.to_file_path().ok())
}

pub fn resolve_list_dirs(
    query: &PathQuery,
    file_dir: Option<&Path>,
    root_dir: Option<&Path>,
    config: &Config,
) -> Vec<PathBuf> {
    match query.prefix_kind {
        PrefixKind::Home => {
            if !config.expand_tilde {
                return Vec::new();
            }
            let home = dirs_home();
            let Some(home) = home else {
                return Vec::new();
            };
            let remainder = query.dir_part.trim_start_matches('~');
            let list_dir = apply_relative_dir(&home, remainder);
            vec![list_dir]
        }
        PrefixKind::Absolute => vec![PathBuf::from(&query.dir_part)],
        PrefixKind::WindowsDrive => vec![PathBuf::from(&query.dir_part)],
        PrefixKind::WindowsUnc => vec![PathBuf::from(&query.dir_part)],
        PrefixKind::Relative => {
            let mut dirs = Vec::new();
            let root = match config.workspace_root_strategy {
                WorkspaceRootStrategy::LspRootUri => root_dir,
                WorkspaceRootStrategy::Disabled => None,
            };
            match config.base_dir {
                BaseDirStrategy::FileDir => {
                    if let Some(dir) = file_dir {
                        dirs.push(apply_relative_dir(dir, &query.dir_part));
                    }
                }
                BaseDirStrategy::WorkspaceRoot => {
                    if let Some(dir) = root {
                        dirs.push(apply_relative_dir(dir, &query.dir_part));
                    }
                }
                BaseDirStrategy::Both => {
                    if let Some(dir) = file_dir {
                        dirs.push(apply_relative_dir(dir, &query.dir_part));
                    }
                    if let Some(dir) = root {
                        let candidate = apply_relative_dir(dir, &query.dir_part);
                        if dirs.iter().all(|d| d != &candidate) {
                            dirs.push(candidate);
                        }
                    }
                }
            }
            dirs
        }
    }
}

fn apply_relative_dir(base: &Path, dir_part: &str) -> PathBuf {
    let mut current = base.to_path_buf();
    for part in dir_part.split(&['/', '\\'][..]) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            }
            continue;
        }
        current = current.join(part);
    }
    current
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

pub fn filter_entries(
    entries: Vec<(String, bool, PathBuf)>,
    segment_prefix: &str,
    config: &Config,
) -> Vec<(String, bool)> {
    let mut filtered: Vec<(String, bool)> = entries
        .into_iter()
        .filter(|(name, is_dir, path)| {
            if !config.show_hidden && name.starts_with('.') {
                return false;
            }
            if !config.include_directories && *is_dir {
                return false;
            }
            if !config.include_files && !*is_dir {
                return false;
            }
            if !name.starts_with(segment_prefix) {
                return false;
            }
            let normalized = normalize_for_match(path);
            !config
                .ignore_globs
                .iter()
                .any(|pattern| glob_match(pattern, &normalized))
        })
        .map(|(name, is_dir, _)| (name, is_dir))
        .collect();
    filtered.sort_by(|(a_name, a_dir), (b_name, b_dir)| {
        b_dir.cmp(a_dir).then_with(|| a_name.cmp(b_name))
    });
    filtered
}

pub fn normalize_for_match(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn glob_match(pattern: &str, text: &str) -> bool {
    let tokens = tokenize_glob(pattern);
    let mut memo = std::collections::HashMap::new();
    glob_match_tokens(&tokens, text.as_bytes(), 0, 0, &mut memo)
}

#[derive(Debug, Clone, Copy)]
enum GlobToken {
    Char(u8),
    Star,
    GlobStar,
}

fn tokenize_glob(pattern: &str) -> Vec<GlobToken> {
    let mut tokens = Vec::new();
    let bytes = pattern.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'*' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                tokens.push(GlobToken::GlobStar);
                i += 2;
                continue;
            }
            tokens.push(GlobToken::Star);
            i += 1;
            continue;
        }
        tokens.push(GlobToken::Char(bytes[i]));
        i += 1;
    }
    tokens
}

fn glob_match_tokens(
    tokens: &[GlobToken],
    text: &[u8],
    ti: usize,
    xi: usize,
    memo: &mut std::collections::HashMap<(usize, usize), bool>,
) -> bool {
    if let Some(&cached) = memo.get(&(ti, xi)) {
        return cached;
    }
    let matched = if ti == tokens.len() {
        xi == text.len()
    } else {
        match tokens[ti] {
            GlobToken::Char(c) => {
                xi < text.len()
                    && text[xi] == c
                    && glob_match_tokens(tokens, text, ti + 1, xi + 1, memo)
            }
            GlobToken::Star => {
                let mut j = xi;
                let mut ok = false;
                while j <= text.len() {
                    if j < text.len() && text[j] == b'/' {
                        break;
                    }
                    if glob_match_tokens(tokens, text, ti + 1, j, memo) {
                        ok = true;
                        break;
                    }
                    j += 1;
                }
                ok
            }
            GlobToken::GlobStar => {
                let mut j = xi;
                let mut ok = false;
                while j <= text.len() {
                    if glob_match_tokens(tokens, text, ti + 1, j, memo) {
                        ok = true;
                        break;
                    }
                    j += 1;
                }
                ok
            }
        }
    };
    memo.insert((ti, xi), matched);
    matched
}

pub fn is_windows_drive_prefix(s: &str) -> bool {
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

pub fn segment_start_offset(content: &str) -> usize {
    let mut last_sep = None;
    for (idx, ch) in content.char_indices() {
        if ch == '/' || ch == '\\' {
            last_sep = Some(idx + ch.len_utf8());
        }
    }
    last_sep.unwrap_or(0)
}

pub fn separator_for_insertion(content_before_cursor: &str, config: &Config) -> char {
    if config.prefer_forward_slashes {
        return '/';
    }
    if content_before_cursor.contains('\\') {
        return '\\';
    }
    std::path::MAIN_SEPARATOR
}

pub fn utf16_len(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_string_context_simple() {
        let line = "open(\"./foo\")";
        let cursor = line.find("./foo").unwrap() + "./foo".len();
        let info = find_string_info(line, cursor).unwrap();
        assert_eq!(info.content_before_cursor, "./foo");
    }

    #[test]
    fn detects_path_query_with_segment() {
        let config = Config::default();
        let query = find_prefix_query("./dir/pa", &config).unwrap();
        assert_eq!(query.dir_part, "./dir/");
        assert_eq!(query.segment_prefix, "pa");
    }

    #[test]
    fn detects_path_query_home() {
        let config = Config::default();
        let query = find_prefix_query("~/Do", &config).unwrap();
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
            ("b.txt".to_string(), false, PathBuf::from("/tmp/b.txt")),
            ("a".to_string(), true, PathBuf::from("/tmp/a")),
            ("a.txt".to_string(), false, PathBuf::from("/tmp/a.txt")),
        ];
        let config = Config::default();
        let filtered = filter_entries(entries, "", &config);
        assert_eq!(filtered[0].0, "a");
        assert!(filtered[0].1);
    }

    #[test]
    fn detects_windows_drive_prefix() {
        assert!(is_windows_drive_prefix("C:\\Users"));
        assert!(is_windows_drive_prefix("D:/Data"));
        assert!(!is_windows_drive_prefix("/tmp"));
    }

    #[test]
    fn segment_start_offset_after_separator() {
        let offset = segment_start_offset("./foo/bar");
        assert_eq!(offset, "./foo/".len());
    }

    #[test]
    fn ignores_fstring_interpolation() {
        let line = "f\"{value}/data\"";
        let cursor_in_expr = line.find("value").unwrap();
        let cursor_in_text = line.find("data").unwrap();
        let info_expr = find_string_info(line, cursor_in_expr);
        let info_text = find_string_info(line, cursor_in_text);
        assert!(info_expr.is_none());
        assert!(info_text.is_some());
    }

    #[test]
    fn glob_match_basic() {
        assert!(glob_match("**/node_modules/**", "/proj/node_modules/pkg"));
        assert!(glob_match("**/.git/**", "/proj/.git/config"));
        assert!(!glob_match("**/.venv/**", "/proj/src/main.py"));
    }
}
