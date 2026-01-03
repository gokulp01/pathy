mod cache;
mod completion;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use cache::{DirCache, DirEntryInfo};
use completion::{
    base_dir_from_uri, filter_and_sort_entries, find_path_query, find_string_context,
    resolve_list_dir,
};
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    InitializeParams, InitializeResult, Position, Range, ServerCapabilities,
    TextDocumentContentChangeEvent, TextDocumentItem, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
};

const MAX_RESULTS: usize = 80;

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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, io_threads) = Connection::stdio();

    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let initialize_params: InitializeParams = serde_json::from_value(initialize_params)?;

    let mut state = ServerState {
        documents: HashMap::new(),
        root_uri: initialize_params.root_uri.clone(),
        cache: DirCache::new(Duration::from_millis(750), 32),
    };

    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["/".into(), ".".into(), "~".into(), "\\".into()]),
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
                handle_notification(&mut state, &notification);
            }
            Message::Response(_) => {}
        }
    }

    io_threads.join()?;
    Ok(())
}

fn handle_notification(state: &mut ServerState, notification: &Notification) {
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
        "initialized" => {}
        "exit" => {
            std::process::exit(0);
        }
        _ => {}
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

    let ctx = match find_string_context(line, cursor_byte) {
        Some(ctx) => ctx,
        None => return Vec::new(),
    };

    let query = match find_path_query(&ctx.content_before_cursor) {
        Some(query) => query,
        None => return Vec::new(),
    };

    let base_dir = base_dir_from_uri(&doc_uri, state.root_uri.as_ref());
    let list_dir = match resolve_list_dir(&query, base_dir.as_deref()) {
        Some(dir) => dir,
        None => return Vec::new(),
    };

    let entries = match list_dir_entries(&list_dir, &mut state.cache) {
        Some(entries) => entries,
        None => return Vec::new(),
    };

    let filtered = filter_and_sort_entries(entries, &query.segment_prefix);

    let segment_len_utf16 = query.segment_prefix.encode_utf16().count() as u32;
    let start = Position {
        line: position.line,
        character: position.character.saturating_sub(segment_len_utf16),
    };
    let range = Range {
        start,
        end: position,
    };

    filtered
        .into_iter()
        .take(MAX_RESULTS)
        .map(|(name, is_dir)| completion_item(name, is_dir, range))
        .collect()
}

fn completion_item(name: String, is_dir: bool, range: Range) -> CompletionItem {
    let mut insert_text = name.clone();
    if is_dir {
        insert_text.push('/');
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

fn list_dir_entries(dir: &PathBuf, cache: &mut DirCache) -> Option<Vec<(String, bool)>> {
    if let Some(cached) = cache.get(dir) {
        return Some(cached.into_iter().map(|e| (e.name, e.is_dir)).collect());
    }

    let mut items: Vec<DirEntryInfo> = Vec::new();
    let read_dir = std::fs::read_dir(dir).ok()?;
    for entry in read_dir.take(MAX_RESULTS * 2) {
        let Ok(entry) = entry else { continue };
        let file_name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        items.push(DirEntryInfo {
            name: file_name,
            is_dir,
        });
    }

    let as_pairs = items
        .iter()
        .map(|item| (item.name.clone(), item.is_dir))
        .collect();
    cache.insert(dir, items);
    Some(as_pairs)
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
