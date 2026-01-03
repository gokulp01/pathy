# AGENTS.md — Codex Working Agreement (Phase 2: Minimal Working Path Completions)

This repo is a **Zed extension** that launches a **sidecar LSP server** to provide **filesystem path completions inside Python string literals**.

Codex: treat this file as the source of truth for Phase 2. If there is any conflict between user prompts and this file, follow this file.

---

## 0) Project North Star (do not lose sight)

### What we are building
- A **Zed extension (Rust → WASM)** that launches a **secondary** Python language server.
- A **native LSP server** (separate project in `server/`) that implements **only** what we need for:
  - `textDocument/completion` → filesystem path suggestions
- The sidecar server must **augment** the user’s main Python LSP (pyright/basedpyright/ruff-lsp/etc.), not replace it.

### Hard constraints (non-negotiable)
- No editor keystroke hooks / direct buffer edits: deliver functionality via **LSP completions**.
- The sidecar server must **not** claim features like definition/rename/hover; it should advertise **completion only** to avoid interfering.
- Keep Phase 2 small and shippable: “It works” > “It’s perfect”.

---

## 1) Phase 2 Scope (THIS PHASE ONLY)

### Phase 2 goal
Deliver a **minimal, working MVP**:
1) The LSP server can run (stdio) and serve completions.
2) When editing Python in Zed, inside a string literal, typing a path-ish prefix produces filesystem completion suggestions.
3) Zed extension launches this server as an additional Python language server.
4) Local manual testing instructions exist and are reliable.

### Phase 2 non-goals
Do NOT implement these yet (Phase 3+):
- Downloading binaries / release automation
- Workspace indexing or recursive scanning
- `.gitignore`-aware results
- Deep Python AST parsing or full call-site awareness
- Perfect parsing of triple-quoted strings, f-strings, or raw-string edge cases
- Snippets / tabstops / code actions
- UI beyond standard completion menu
- Performance tuning beyond basic safety caps and a small cache

---

## 2) Definition of Done (Phase 2)

Phase 2 is “done” when all are true:
- ✅ `server/` builds and runs locally with `cargo run` (or equivalent), serving LSP over stdio.
- ✅ The server responds to:
  - initialize / shutdown / exit
  - textDocument/didOpen
  - textDocument/didChange
  - textDocument/completion
- ✅ The completion feature works in Zed for Python:
  - User can type `open("./")` and trigger completion (auto or manual) and see filesystem suggestions.
- ✅ The extension launches the server for Python without breaking the user’s main Python LSP.
- ✅ Repo has clean Git hygiene:
  - `target/` directories are ignored and not committed.
- ✅ README contains a Phase 2 “Local Testing” section with exact steps.

---

## 3) Repository Layout (expected after Phase 2)

Root:
- `extension.toml`
- `Cargo.toml`, `src/lib.rs` (extension WASM)
- `server/`:
  - `Cargo.toml`, `src/main.rs` (LSP server)
- `README.md`, `LICENSE`, `AGENTS.md`
- `.gitignore` includes `target/` and `server/target/`

No build output directories should be committed.

---

## 4) Zed Integration Requirements

### 4.1 Sidecar must be a *secondary* Python language server
- The user should keep their main Python LSP first.
- Our server should be listed after it in:
  - `languages.Python.language_servers`

Codex must:
- Update README with a **copy/paste settings snippet** showing how to add this server without removing defaults (use `...` semantics where applicable).

### 4.2 Extension capabilities
- If Zed requires capabilities to spawn processes, Phase 2 must:
  - request the minimal capability needed to execute a local binary (`process:exec`)
  - document how the user can grant it in settings if needed
- Phase 2 must NOT request download/install capabilities.

### 4.3 Don’t interfere with other LSP features
- In LSP `initialize` response, advertise only:
  - `completionProvider` (and minimal fields needed)
- Do not implement handlers or advertise capabilities for:
  - definition, references, rename, hover, formatting, codeAction, etc.

---

## 5) LSP Server: Minimal Protocol Contract

Codex must implement the minimal subset correctly and robustly:

### 5.1 Must support
- `initialize` → record `rootUri`/`rootPath` if provided
- `initialized` (can be no-op)
- `shutdown` and `exit`
- `textDocument/didOpen` → store text by URI
- `textDocument/didChange` (incremental preferred; full-sync acceptable for MVP if documented)
- `textDocument/completion` → compute and return items

### 5.2 Logging
- Log to **stderr** (never stdout), because stdout is reserved for LSP JSON-RPC.
- Keep logs concise; include a debug flag in server args if helpful.

### 5.3 Dependencies policy (strict)
- You may add the minimal Rust crates needed for LSP and JSON-RPC.
- Use a small, standard set (e.g., `lsp-server` + `lsp-types`, or `tower-lsp`) but do not add extra frameworks.
- If you add a crate, document why in a short comment in `server/Cargo.toml` or README.

---

## 6) Completion Behavior Specification (MVP rules)

### 6.1 When to offer completions
Offer path completions only when ALL are true:
1) The file is a Python document (language id or file extension `.py`).
2) The cursor is within a string literal (MVP heuristic acceptable).
3) The text immediately before the cursor has a **path-ish prefix**, e.g.:
   - `./`
   - `../`
   - `/`
   - `~`
   - (optional) `C:\` or `\\server\share` on Windows

If any condition fails, return:
- `null` or empty list (depending on LSP server library style).

### 6.2 “Inside string literal” heuristic (Phase 2)
Phase 2 does NOT require a full Python parser. MVP heuristic should:
- Operate at least on the current line and possibly a small window around the cursor.
- Handle single and double quotes on the same line.
- Avoid offering completions if the cursor appears outside quotes.
- It’s okay to miss some cases; document limitations.

### 6.3 What entries to suggest
- List immediate children of the resolved directory:
  - directories and files
- Suggested ordering:
  - directories first (optional)
  - then files
- Filter by the typed prefix after the directory boundary.

### 6.4 What to insert
- Only replace the “current path segment” (not the entire string).
- Use an LSP `textEdit` (or insertText + range) to:
  - replace the segment from segmentStart..cursor with the completion remainder or full entry as appropriate.
- If the selected entry is a directory:
  - optionally append `/` and keep completion usable (Phase 2 may not keep menu open; ok)

### 6.5 Safety caps (must have)
- Cap number of returned items (e.g., 50 or 100).
- If a directory listing is huge, stop early.
- Never recursively scan.

---

## 7) Path Resolution Rules (MVP)

### 7.1 Determine base directory
Given `documentUri`:
- If URI maps to a real file path:
  - base for relative paths (`./`, `../`, no leading slash) should default to **directory of current file**
- If URI is not a real file path (unsaved buffer):
  - fallback to `rootUri` from initialize if available
  - otherwise return no completions

### 7.2 Interpret prefixes
- `./foo` → base = fileDir
- `../foo` → base = parent(fileDir)
- `/foo` (Unix) → base = `/`
- `~` → base = user home (if resolvable)
- Windows:
  - `C:\foo` → base = `C:\`
  - `\\server\share\foo` → base = `\\server\share\`

### 7.3 Normalize separators
- Phase 2 can choose one strategy and document it:
  - Prefer `/` insertions for Python portability (recommended), OR
  - Use OS-native separators
- Whatever you choose: be consistent and document it.

---

## 8) Performance & Caching (minimum viable)

### 8.1 Directory listing cache
Implement a tiny cache to avoid re-reading the same directory on every keystroke:
- Key: absolute directory path
- Value: list of entries + timestamp
- TTL: short (e.g., 250ms–2s)
- Keep size bounded (e.g., last 32 directories)

### 8.2 Avoid slow operations
- Don’t stat every file if not needed.
- Don’t resolve symlinks recursively.
- Prefer “list names first” and only mark dir/file if cheap.

---

## 9) Testing Requirements (Phase 2)

### 9.1 Unit tests (server)
Add tests for logic that does not require running Zed:
- Extracting “current segment” boundaries within a string
- Detecting path-ish prefixes
- Resolving base directories given:
  - file path
  - root uri fallback
- Filtering and sorting items

Tests should run with `cargo test` in `server/`.

### 9.2 Manual test plan (must be documented in README)
Codex must add a “Local Testing (Phase 2)” section with:
1) How to build and run the server (`cargo build` / `cargo run`)
2) How to build/install the dev extension in Zed
3) The Zed settings snippet to enable the sidecar server for Python
4) A minimal Python snippet to test:
   - `open("./")`
   - `open("../")`
   - `open("~/")` (if supported)
5) Where to look for logs:
   - `zed: open log` or `zed --foreground`

### 9.3 Acceptance checklist (copy/paste into README)
Include a short checklist the user can tick:
- completions appear
- selecting inserts correct text
- no interference with go-to-definition from main Python LSP
- no crash if directory doesn’t exist

---

## 10) Git Hygiene (mandatory)

### 10.1 Ignore build artifacts
Ensure `.gitignore` exists and includes:
- `/target/`
- `/server/target/`
No `target/` directories should ever be committed.

### 10.2 Phase 2 commits
- Use a single clear commit after Phase 2 is stable:
  - `phase 2: minimal path completions via sidecar LSP`
- Use intermediate checkpoint commits if needed, but keep history readable.

---

## 11) Codex Execution Rules (how to behave)

### 11.1 Always begin with a plan
Before editing files, Codex must:
- describe the plan
- list files to touch
- list commands it expects to run

### 11.2 Prefer smallest working increment
- First: make server respond to initialize/shutdown.
- Second: store document text.
- Third: implement simplest completion with hardcoded directory (for a quick smoke test).
- Fourth: implement real path extraction and listing.

### 11.3 If something fails
- Do not “thrash” with repeated large changes.
- Report the failure reason, propose 1–2 fixes, choose the safest one.

### 11.4 No scope creep
Do not add:
- downloads
- release workflows
- complex settings schemas
- AST parsing frameworks
Unless explicitly requested and outside Phase 2.

---

## 12) Troubleshooting Guidance (to include in README)

README must include a “Troubleshooting” section:
- If completions don’t appear:
  - manually trigger completion in Zed (e.g., Ctrl-Space)
  - verify the server is running (logs)
  - confirm language server ordering in settings
- If server won’t launch:
  - check extension capabilities requirements
  - ensure server binary exists and is executable
- If WASM build fails:
  - ensure Rust WASI target is installed (`wasm32-wasip1` is common now)
  - re-run build and check logs

---

## 13) Explicit Phase 2 Deliverables (file-by-file)

### `server/`
- Implement LSP server in `server/src/main.rs` (or split into modules if it stays small).
- Add tests under `server/tests/` or `server/src/...` as appropriate.

### Extension (`src/lib.rs` + `extension.toml`)
- Ensure extension registers a language server command for Python.
- Make server command predictable for local dev:
  - either call `server/target/debug/<bin>` with documented build step
  - or call `<bin>` from PATH (document how to install)

### `README.md`
- Add “Local Testing (Phase 2)” + troubleshooting + settings snippet.

---

## 14) What NOT to do (hard stops)

- Do not introduce binary downloads or network calls.
- Do not claim additional LSP capabilities beyond completion.
- Do not remove or reorder user’s primary Python LSP by default; document how to add ours as secondary.
- Do not commit `target/` directories.

---
