mod cache;
mod completion;
mod config;
mod context;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use cache::{DirCache, DirEntryInfo};
use completion::{
    base_dir_from_uri, build_relative_query, filter_entries, find_prefix_query, find_string_info,
    resolve_list_dirs, segment_start_offset, separator_for_insertion, utf16_len,
};
use config::{load_config, Config, ContextGating};
use context::is_path_context;
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    ConfigurationItem, ConfigurationParams, InitializeParams, InitializeResult, Position, Range,
    ServerCapabilities, TextDocumentContentChangeEvent, TextDocumentItem,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};

#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
    language_id: Option<String>,
}

#[derive(Debug)]
struct ServerState {
    documents: HashMap<Url, DocumentState>,
    root_uri: Option<Url>,
    cache: DirCache,
    config: Config,
    config_warned: bool,
    debug: bool,
    pending_config_request: Option<RequestId>,
    next_request_id: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, io_threads) = Connection::stdio();

    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let initialize_params: InitializeParams = serde_json::from_value(initialize_params)?;

    let mut config_warned = false;
    let mut config = Config::default();
    if let Some(options) = initialize_params.initialization_options.as_ref() {
        config = load_config(options, &mut config_warned);
    }

    let debug = std::env::var_os("PATHY_DEBUG").is_some();

    let mut state = ServerState {
        documents: HashMap::new(),
        root_uri: initialize_params.root_uri.clone(),
        cache: DirCache::new(
            Duration::from_millis(config.cache_ttl_ms),
            config.cache_max_dirs,
        ),
        config,
        config_warned,
        debug,
        pending_config_request: None,
        next_request_id: 1,
    };

    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["/".into(), "\\".into(), "~".into(), ".".into()]),
            resolve_provider: Some(false),
            ..CompletionOptions::default()
        }),
        ..ServerCapabilities::default()
    };

    let initialize_result = InitializeResult {
        capabilities,
        server_info: Some(lsp_types::ServerInfo {
            name: "pathy-server".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        }),
    };

    connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    break;
                }
                handle_request(&connection, &mut state, &request);
            }
            Message::Notification(notification) => {
                handle_notification(&connection, &mut state, &notification);
            }
            Message::Response(response) => {
                handle_response(&mut state, &response);
            }
        }
    }

    io_threads.join()?;
    Ok(())
}

fn handle_notification(
    connection: &Connection,
    state: &mut ServerState,
    notification: &Notification,
) {
    match notification.method.as_str() {
        "textDocument/didOpen" => {
            if let Ok(params) = serde_json::from_value::<lsp_types::DidOpenTextDocumentParams>(
                notification.params.clone(),
            ) {
                let TextDocumentItem {
                    uri,
                    text,
                    language_id,
                    ..
                } = params.text_document;
                state.documents.insert(
                    uri,
                    DocumentState {
                        text,
                        language_id: Some(language_id),
                    },
                );
            }
        }
        "textDocument/didChange" => {
            if let Ok(params) = serde_json::from_value::<lsp_types::DidChangeTextDocumentParams>(
                notification.params.clone(),
            ) {
                if let Some(doc) = state.documents.get_mut(&params.text_document.uri) {
                    if let Some(TextDocumentContentChangeEvent { text, .. }) =
                        params.content_changes.last().cloned()
                    {
                        doc.text = text;
                    }
                }
            }
        }
        "workspace/didChangeConfiguration" => {
            if let Ok(params) = serde_json::from_value::<lsp_types::DidChangeConfigurationParams>(
                notification.params.clone(),
            ) {
                apply_config_update(state, &params.settings);
            }
        }
        "initialized" => {
            request_workspace_config(connection, state);
        }
        "exit" => {
            std::process::exit(0);
        }
        _ => {}
    }
}

fn handle_response(state: &mut ServerState, response: &Response) {
    if let Some(pending) = &state.pending_config_request {
        if &response.id == pending {
            state.pending_config_request = None;
            if let Some(result) = response.result.as_ref() {
                if let Some(list) = result.as_array() {
                    if let Some(value) = list.first() {
                        apply_config_update(state, value);
                        return;
                    }
                }
                apply_config_update(state, result);
            }
        }
    }
}

fn request_workspace_config(connection: &Connection, state: &mut ServerState) {
    let id = RequestId::from(state.next_request_id);
    state.next_request_id += 1;
    state.pending_config_request = Some(id.clone());

    let params = ConfigurationParams {
        items: vec![ConfigurationItem {
            scope_uri: None,
            section: Some("pathy".into()),
        }],
    };

    let request = Request::new(id, "workspace/configuration".into(), params);
    connection.sender.send(Message::Request(request)).ok();
}

fn apply_config_update(state: &mut ServerState, value: &serde_json::Value) {
    let new_config = load_config(value, &mut state.config_warned);
    state.cache.update_limits(
        Duration::from_millis(new_config.cache_ttl_ms),
        new_config.cache_max_dirs,
    );
    state.config = new_config;
    if state.debug {
        eprintln!("pathy-server: config updated");
    }
}

fn handle_request(connection: &Connection, state: &mut ServerState, request: &Request) {
    match request.method.as_str() {
        "textDocument/completion" => {
            let params = match serde_json::from_value::<CompletionParams>(request.params.clone()) {
                Ok(params) => params,
                Err(_) => {
                    let response = Response::new_err(
                        request.id.clone(),
                        lsp_server::ErrorCode::InvalidParams as i32,
                        "Invalid completion params".into(),
                    );
                    connection.sender.send(Message::Response(response)).ok();
                    return;
                }
            };
            let items = completion_items(state, params);
            let result = CompletionResponse::Array(items);
            let response = Response::new_ok(request.id.clone(), result);
            connection.sender.send(Message::Response(response)).ok();
        }
        _ => {
            let response = Response::new_err(
                request.id.clone(),
                lsp_server::ErrorCode::MethodNotFound as i32,
                "Method not found".into(),
            );
            connection.sender.send(Message::Response(response)).ok();
        }
    }
}

fn completion_items(state: &mut ServerState, params: CompletionParams) -> Vec<CompletionItem> {
    if !state.config.enable {
        return Vec::new();
    }

    let doc_uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let doc = match state.documents.get(&doc_uri) {
        Some(doc) => doc.clone(),
        None => return Vec::new(),
    };

    if !is_python_document(&doc_uri, doc.language_id.as_deref()) {
        return Vec::new();
    }

    let line = match get_line(&doc.text, position.line) {
        Some(line) => line,
        None => return Vec::new(),
    };

    let cursor_byte = match utf16_col_to_byte(line, position.character) {
        Some(idx) => idx,
        None => return Vec::new(),
    };

    let line_start_offset = match line_start_offset(&doc.text, position.line) {
        Some(offset) => offset,
        None => return Vec::new(),
    };

    let info = match find_string_info(line, cursor_byte) {
        Some(info) => info,
        None => return Vec::new(),
    };

    let string_start_offset = line_start_offset + info.string_start_byte;

    let prefix_query = if state.config.path_prefix_fallback {
        find_prefix_query(&info.content_before_cursor, &state.config)
    } else {
        None
    };

    if !is_completion_allowed(
        state,
        &doc.text,
        prefix_query.is_some(),
        string_start_offset,
    ) {
        log_debug(state, "completion gated off");
        return Vec::new();
    }

    let query = prefix_query.unwrap_or_else(|| build_relative_query(&info.content_before_cursor));

    let file_dir = base_dir_from_uri(&doc_uri, None);
    let root_dir = state
        .root_uri
        .as_ref()
        .and_then(|uri| uri.to_file_path().ok());

    let list_dirs = resolve_list_dirs(
        &query,
        file_dir.as_deref(),
        root_dir.as_deref(),
        &state.config,
    );
    if list_dirs.is_empty() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    for dir in list_dirs {
        if let Some(mut listed) = list_dir_entries(&dir, &mut state.cache, &state.config) {
            entries.append(&mut listed);
        }
    }

    let filtered = filter_entries(entries, &query.segment_prefix, &state.config);

    let segment_start_byte = segment_start_offset(&info.content_before_cursor);
    let segment_start_utf16 = utf16_len(&info.content_before_cursor[..segment_start_byte]);

    let start = Position {
        line: position.line,
        character: info.string_start_utf16 + segment_start_utf16,
    };
    let range = Range {
        start,
        end: position,
    };

    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for (name, is_dir) in filtered.into_iter().take(state.config.max_results) {
        if seen.insert(name.clone()) {
            deduped.push((name, is_dir));
        }
    }

    deduped
        .into_iter()
        .map(|(name, is_dir)| completion_item(name, is_dir, range, &state.config, &info))
        .collect()
}

fn is_completion_allowed(
    state: &ServerState,
    text: &str,
    has_prefix_fallback: bool,
    string_start_offset: usize,
) -> bool {
    match state.config.context_gating {
        ContextGating::Strict => is_path_context(text, string_start_offset),
        ContextGating::Off => true,
        ContextGating::Smart => {
            if has_prefix_fallback {
                true
            } else {
                is_path_context(text, string_start_offset)
            }
        }
    }
}

fn completion_item(
    name: String,
    is_dir: bool,
    range: Range,
    config: &Config,
    info: &completion::StringInfo,
) -> CompletionItem {
    let mut insert_text = name.clone();
    if is_dir && config.directory_trailing_slash {
        let sep = separator_for_insertion(&info.content_before_cursor, config);
        insert_text.push(sep);
    }
    CompletionItem {
        label: name,
        kind: Some(if is_dir {
            CompletionItemKind::FOLDER
        } else {
            CompletionItemKind::FILE
        }),
        text_edit: Some(lsp_types::CompletionTextEdit::Edit(lsp_types::TextEdit {
            range,
            new_text: insert_text,
        })),
        ..CompletionItem::default()
    }
}

fn list_dir_entries(
    dir: &PathBuf,
    cache: &mut DirCache,
    config: &Config,
) -> Option<Vec<(String, bool, PathBuf)>> {
    if let Some(cached) = cache.get(dir) {
        let entries = cached
            .into_iter()
            .map(|e| {
                let name = e.name;
                let path = dir.join(&name);
                (name, e.is_dir, path)
            })
            .collect();
        return Some(entries);
    }

    let mut items: Vec<DirEntryInfo> = Vec::new();
    let mut entries: Vec<(String, bool, PathBuf)> = Vec::new();
    let read_dir = std::fs::read_dir(dir).ok()?;
    for entry in read_dir.take(config.max_results * 2) {
        let Ok(entry) = entry else { continue };
        let file_name = entry.file_name().to_string_lossy().to_string();
        let is_dir = match config.stat_strategy {
            config::StatStrategy::None => false,
            _ => entry.file_type().map(|t| t.is_dir()).unwrap_or(false),
        };
        items.push(DirEntryInfo {
            name: file_name.clone(),
            is_dir,
        });
        entries.push((file_name, is_dir, dir.join(entry.file_name())));
    }

    cache.insert(dir, items);
    Some(entries)
}

fn is_python_document(uri: &Url, language_id: Option<&str>) -> bool {
    if let Some(lang) = language_id {
        if lang.eq_ignore_ascii_case("python") {
            return true;
        }
    }
    uri.path().ends_with(".py")
}

fn get_line(text: &str, line: u32) -> Option<&str> {
    text.split('\n').nth(line as usize)
}

fn line_start_offset(text: &str, line: u32) -> Option<usize> {
    let mut offset = 0usize;
    let mut current = 0u32;
    for part in text.split('\n') {
        if current == line {
            return Some(offset);
        }
        offset += part.len() + 1;
        current += 1;
    }
    None
}

fn utf16_col_to_byte(line: &str, col: u32) -> Option<usize> {
    let mut count = 0u32;
    for (idx, ch) in line.char_indices() {
        let next = count + ch.len_utf16() as u32;
        if next > col {
            return Some(idx);
        }
        count = next;
        if count == col {
            return Some(idx + ch.len_utf8());
        }
    }
    if count == col {
        return Some(line.len());
    }
    None
}

fn log_debug(state: &ServerState, message: &str) {
    if state.debug {
        eprintln!("pathy-server: {}", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_start_offset_basic() {
        let text = "a\nb\nc";
        assert_eq!(line_start_offset(text, 1), Some(2));
        assert_eq!(line_start_offset(text, 2), Some(4));
    }

    #[test]
    fn replacement_range_uses_segment_start() {
        let info = completion::StringInfo {
            content_before_cursor: "./foo/bar".into(),
            is_raw: false,
            is_fstring: false,
            string_start_byte: 6,
            string_start_utf16: 6,
        };
        let segment_start = segment_start_offset(&info.content_before_cursor);
        let utf16 = utf16_len(&info.content_before_cursor[..segment_start]);
        assert_eq!(utf16, "./foo/".len() as u32);
    }
}
