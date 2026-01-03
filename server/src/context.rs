#[derive(Debug, Clone)]
pub struct CallContext {
    pub full_name: String,
    pub base_name: String,
    pub arg_is_first: bool,
    pub named_arg: Option<String>,
}

pub fn detect_call_context(text: &str, string_start_offset: usize) -> Option<CallContext> {
    let window_start = string_start_offset.saturating_sub(300);
    let window = &text[window_start..string_start_offset];
    let mut depth = 0i32;
    let mut open_idx = None;

    let indices: Vec<(usize, char)> = window.char_indices().collect();
    for (idx, ch) in indices.iter().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                if depth == 0 {
                    open_idx = Some(*idx);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
    }

    let open_idx = open_idx?;
    let before = window[..open_idx].trim_end();
    let arg_text = window[open_idx + 1..].to_string();

    let name_start = before
        .char_indices()
        .rev()
        .find(|(_, ch)| !is_name_char(*ch))
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or(0);
    let full_name = before[name_start..].to_string();
    if full_name.is_empty() {
        return None;
    }
    let base_name = full_name
        .rsplit('.')
        .next()
        .unwrap_or(&full_name)
        .to_string();

    let arg_info = analyze_arg_text(&arg_text);

    Some(CallContext {
        full_name,
        base_name,
        arg_is_first: arg_info.0,
        named_arg: arg_info.1,
    })
}

pub fn is_path_context(text: &str, string_start_offset: usize) -> bool {
    if let Some(ctx) = detect_call_context(text, string_start_offset) {
        if !ctx.arg_is_first && ctx.named_arg.is_none() {
            return false;
        }

        if matches_known_path_function(&ctx.full_name, &ctx.base_name) {
            return true;
        }

        if let Some(name) = ctx.named_arg.as_deref() {
            if matches_named_path_arg(name) {
                return true;
            }
        }
    }

    if path_join_operator_context(text, string_start_offset) {
        return true;
    }

    false
}

fn analyze_arg_text(arg_text: &str) -> (bool, Option<String>) {
    let trimmed = arg_text.trim();
    if trimmed.is_empty() {
        return (true, None);
    }
    if trimmed.contains(',') {
        return (false, None);
    }
    if let Some(eq_pos) = trimmed.rfind('=') {
        let name = trimmed[..eq_pos].trim();
        if !name.is_empty() {
            return (false, Some(name.to_string()));
        }
    }
    (false, None)
}

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
}

fn matches_known_path_function(full: &str, base: &str) -> bool {
    matches!(base, "open" | "Path")
        || matches!(
            base,
            "read_csv" | "read_parquet" | "read_json" | "read_excel" | "read_table"
        )
        || full.ends_with(".read_csv")
        || full.ends_with(".read_parquet")
        || full.ends_with(".read_json")
        || full.ends_with(".read_excel")
        || full.ends_with(".read_table")
        || full.ends_with(".Path")
}

fn matches_named_path_arg(name: &str) -> bool {
    matches!(name, "path" | "filepath" | "filename" | "file" | "fname")
}

fn path_join_operator_context(text: &str, string_start_offset: usize) -> bool {
    let window_start = string_start_offset.saturating_sub(120);
    let window = &text[window_start..string_start_offset];
    let path_pos = window.rfind("Path(");
    let path_mod_pos = window.rfind("pathlib.Path(");
    let path_start = path_pos.or(path_mod_pos);
    let Some(path_start) = path_start else {
        return false;
    };
    let after = &window[path_start..];
    after.contains("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_open_context() {
        let text = "with open(\"./foo\") as f:";
        let offset = text.find('\"').unwrap();
        assert!(is_path_context(text, offset));
    }

    #[test]
    fn detects_pathlib_context() {
        let text = "Path(\"./foo\")";
        let offset = text.find('\"').unwrap();
        assert!(is_path_context(text, offset));
    }

    #[test]
    fn detects_pandas_context() {
        let text = "pandas.read_csv(\"data.csv\")";
        let offset = text.find('\"').unwrap();
        assert!(is_path_context(text, offset));
    }

    #[test]
    fn ignores_non_path_context() {
        let text = "print(\"hello\")";
        let offset = text.find('\"').unwrap();
        assert!(!is_path_context(text, offset));
    }

    #[test]
    fn allows_named_path_arg() {
        let text = "load_data(path=\"./data.csv\")";
        let offset = text.find('\"').unwrap();
        assert!(is_path_context(text, offset));
    }
}
