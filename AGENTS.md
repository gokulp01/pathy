# AGENTS.md — Codex Working Agreement (Phase 3: “Editor-Quality” Path Intellisense)

This repository contains:
- A **Zed extension** (Rust → WASM) that launches a sidecar Language Server.
- A **sidecar LSP server** (in `server/`) that provides **filesystem path completions inside Python string literals**.

Codex: treat this file as the source of truth for **Phase 3**. If a user prompt conflicts with this file, follow this file.

---

## 0) Project North Star (do not drift)

### What we are building
We are building a **Zed-native, LSP-based Path Intellisense** experience for Python:
- When the user types inside a Python string that represents a file path, they get completion suggestions from the filesystem.
- The experience should feel “obvious and dependable”:
  - works in the common cases without manual babysitting
  - avoids noisy suggestions in non-path strings
  - respects user preferences (base dir, ignore patterns, hidden files, separators, etc.)
  - remains fast (no recursion, no full indexing)

### Core constraints (non-negotiable)
1) **No editor keypress hooks / no direct buffer edits**: only LSP completion.
2) The sidecar server is **secondary** and must **not interfere** with primary Python LSP (pyright/basedpyright/ruff-lsp).
3) The server must **advertise completion-only capability** (no hover/definition/rename/code actions).
4) Phase 3 does **not** include distribution automation or binary downloads (that is Phase 4).

---

## 1) Phase 3 Scope (THIS PHASE ONLY)

### Phase 3 goal
Upgrade the Phase 2 MVP into an “editor-quality” feature by adding:

A) **Context awareness (“smart gating”)**
- Only show filesystem path completions when the string is likely a filepath:
  - common path-taking functions and constructors (e.g., `open(...)`, `Path(...)`, `read_*` APIs)
  - common library calls (e.g., pandas `read_csv`, `read_parquet`, etc.)
- Still allow a fallback: if the user types a clearly path-like prefix (`./`, `../`, `/`, `~`, drive/UNC), show completions anywhere in a string.

B) **User configuration support**
Expose settings that users can set via Zed’s LSP settings mechanism (Phase 2 already had a basic setup; Phase 3 makes it complete, documented, and stable):
- base directory strategy
- ignore patterns
- show hidden files
- max results
- separator behavior (prefer `/` vs OS-native)
- directory insertion behavior (trailing slash)
- trigger behavior

C) **Robustness and cross-platform correctness**
- Better string literal detection (without needing a full Python parser in Phase 3)
- More complete Windows prefix handling (drive letters, UNC paths), while keeping portable defaults
- Better path segment extraction and replacement ranges
- Better behavior in unsaved files and multi-root workspaces

D) **Performance and reliability improvements**
- More resilient caching (bounded, TTL, safe invalidation)
- Avoid expensive I/O and huge directory listings
- Deterministic sorting and filtering

E) **Testing + docs**
- Expand unit tests to cover config parsing, context gating, Windows cases, and segment extraction.
- Improve README with:
  - configuration reference
  - “why/when completions appear”
  - troubleshooting
  - known limitations (clearly)

### Phase 3 non-goals (hard stops)
Do NOT implement yet:
- Downloading server binaries / GitHub Releases / CI distribution
- Recursive indexing / background crawling
- `.gitignore` parsing (optional future; not required in Phase 3)
- Full Python AST parsing via heavy frameworks unless absolutely necessary
- Snippet/tabstop insertion (plain completion is fine)
- UI panes, trees, or non-standard Zed UI integrations
- Any LSP features beyond completion

---

## 2) Definition of Done (Phase 3)

Phase 3 is “done” when all items are true:

### Functionality
- ✅ Path completions appear in Python strings in “obvious path contexts” without requiring explicit prefixes.
- ✅ Path completions still appear anywhere if the user types a clearly path-like prefix.
- ✅ Completions are not annoyingly noisy in arbitrary non-path strings.

### Configuration
- ✅ Documented user settings work (Zed settings → server config).
- ✅ Changing config (restart or configuration change) produces expected behavior.
- ✅ Default settings are sensible and safe.

### Correctness
- ✅ Correct replacement range: only the current path segment is replaced.
- ✅ Reasonable handling of:
  - `./`, `../`, `/`, `~`
  - Windows drive prefixes (e.g., `C:\`) and UNC (e.g., `\\server\share`)
- ✅ No recursion; listing is bounded; server remains responsive.

### Integration
- ✅ Sidecar remains completion-only; does not interfere with “go to definition” and other features from primary Python LSP.
- ✅ Zed extension still launches server reliably with minimal capabilities (no downloads).

### Quality
- ✅ Expanded unit tests pass.
- ✅ README updated with:
  - configuration reference
  - examples
  - troubleshooting
  - limitations
- ✅ Git hygiene: no `target/` artifacts committed; `.gitignore` remains correct.

---

## 3) Expected Repo Layout (Phase 3)

Root:
- `extension.toml`
- `Cargo.toml`, `src/lib.rs`
- `.gitignore`
- `README.md`, `LICENSE`, `AGENTS.md`

Server:
- `server/Cargo.toml`, `server/src/...`
- `server` modules may be split (e.g., `completion`, `config`, `context`, `cache`, `paths`) as long as it stays small and readable.

No build output directories committed.

---

## 4) Codex Working Rules (apply always)

### 4.1 Start with a plan
Before editing files, Codex must:
1) Summarize the task
2) List files to change/create
3) Outline steps in the order they will be implemented
4) State assumptions and confirm constraints (completion-only, no Phase 4 features)

### 4.2 Keep diffs small and focused
- Prefer incremental PR-sized changes even within Phase 3.
- Avoid refactors unrelated to gating/config/correctness.
- Do not churn formatting in unrelated files.

### 4.3 No surprise dependencies
- Do not add new crates unless needed.
- If adding a crate:
  - justify why it is necessary
  - prefer lightweight, widely-used crates
  - avoid heavy parsing frameworks unless absolutely needed

### 4.4 Always validate
- Run formatting and tests.
- If something fails, report precisely why and propose minimal fixes.

### 4.5 No secrets / no credentials
- Never request or embed tokens.
- No network calls required for Phase 3.

---

## 5) Configuration Contract (Phase 3)

### 5.1 Where config comes from
The server must be configurable via Zed’s LSP settings mechanism.
Implementation MUST support at least one of:
- `workspace/didChangeConfiguration` (preferred)
- `workspace/configuration` request (if client uses it)
- `initialize.initializationOptions` (fallback)

Codex must implement a robust approach:
- On initialize: set defaults
- Then attempt to read user settings if provided
- Support updates (if Zed sends configuration changes)

### 5.2 Config schema (MUST document in README)
Define settings with stable names and defaults. Recommended keys (you may adjust names, but keep them stable once chosen):

#### Behavior / gating
- `enable`: bool (default: true)
- `path_prefix_fallback`: bool (default: true)
- `context_gating`: enum (default: "smart")
  - "off": always show completions in any string when manual trigger occurs
  - "smart": show in known contexts + prefix fallback
  - "strict": only known contexts, no fallback (only if user opts in)

#### Base directory
- `base_dir`: enum (default: "file_dir")
  - "file_dir": directory of the current file
  - "workspace_root": worktree root
  - "both": offer both (implementation: merge lists or prefer one)
- `workspace_root_strategy`: enum (default: "lsp_root_uri")
  - "lsp_root_uri": use initialize rootUri
  - "file_parent_chain": walk up from file until repo markers (optional, Phase 3 can skip)
  - "disabled": never use workspace root

#### Listing and filtering
- `max_results`: int (default: 80)
- `show_hidden`: bool (default: false)
- `include_files`: bool (default: true)
- `include_directories`: bool (default: true)
- `directory_trailing_slash`: bool (default: true)
- `ignore_globs`: array[string] (default: common ignores below)
  - Suggested defaults:
    - "**/.git/**"
    - "**/.venv/**"
    - "**/venv/**"
    - "**/__pycache__/**"
    - "**/.pytest_cache/**"
    - "**/.mypy_cache/**"
    - "**/.ruff_cache/**"
    - "**/node_modules/**"

#### Paths and separators
- `prefer_forward_slashes`: bool (default: true)
- `expand_tilde`: bool (default: true)
- `windows_enable_drive_prefix`: bool (default: true)
- `windows_enable_unc`: bool (default: true)

#### Performance / cache
- `cache_ttl_ms`: int (default: 500)
- `cache_max_dirs`: int (default: 64)
- `stat_strategy`: enum (default: "lazy")
  - "none": don’t stat; treat unknown as file
  - "lazy": stat only needed entries (recommended)
  - "eager": stat all (avoid unless necessary)

### 5.3 Backwards compatibility
- If Phase 2 already documented certain keys, keep them working.
- Unknown keys must be ignored without crashing.
- Invalid values must fall back to defaults with a logged warning (stderr).

---

## 6) Completion Behavior Spec (Phase 3)

### 6.1 High-level algorithm (must remain deterministic)
On `textDocument/completion`:
1) Load document text for the URI (must exist; otherwise return none).
2) Determine if the cursor is inside a Python string literal (heuristic).
3) Extract:
   - the string content around cursor
   - the “current path segment” and its replacement range
4) Decide if completion should be offered:
   - If prefix fallback matches: offer
   - Else if context gating matches known call-sites: offer
   - Else: return none
5) Resolve base directory:
   - according to `base_dir` setting and the detected prefix type
6) List directory entries (bounded, cached)
7) Filter by typed prefix in the segment
8) Convert to LSP completion items
9) Return results (bounded)

### 6.2 String literal detection (Phase 3 target)
Phase 2 was line-based and limited. Phase 3 should improve while staying lightweight:

Minimum improvements:
- handle escaped quotes on the same line
- handle raw strings heuristically (`r"..."`, `r'...'`) for escaping decisions
- handle f-strings heuristically:
  - completions should apply only in literal portions (not inside `{...}`) in Phase 3 if feasible
- triple quotes are optional but recommended if not too hard:
  - simplest acceptable approach: if the cursor is within a triple-quoted string on the same line as opening delimiter, treat as string; otherwise document limitation

Be honest in README about what is and isn’t supported.

### 6.3 Context gating rules (“smart” mode)
Context gating is a filter: only show completions in likely-path argument strings.

Implement a pragmatic approach:
- Identify the immediate call expression / function name near the cursor
- If the cursor is inside the first positional arg or a named arg known to be a path, allow completions

Phase 3 must include at least these canonical contexts:
- Builtins / stdlib:
  - `open(...)`
  - `Path(...)` (from `pathlib`)
  - `os.path.*` functions that take paths (optional)
- Common patterns:
  - `with open("...") as f:`
  - `Path("...") / "child"` (if cursor is in the string literal)
- Common libraries (minimal list; configurable in the future):
  - `pandas.read_csv`, `read_parquet`, `read_json`, `read_excel` (best-effort)

Additionally:
- Always allow when string prefix is path-like (fallback).

### 6.4 Path-like prefix fallback (must remain)
Regardless of context gating, if the prefix before cursor matches:
- `./`, `../`, `/`, `~`
- Windows:
  - drive letter + `:\` or `:/`
  - UNC `\\server\share\`
then completions are allowed.

### 6.5 Segment extraction and replacement range (Phase 3 target)
- Replacement should start at the beginning of the current path segment inside the string.
- It must not overwrite:
  - the opening quote
  - earlier parts of the string not part of the segment
- It must not insert outside the string.

Define clear segment boundaries:
- Segment begins after the last path separator (`/` or `\`) in the string content before cursor
- Segment ends at cursor (Phase 3 can ignore selection/range expansions)

### 6.6 Insert behavior
- Directory entries:
  - insert trailing `/` (configurable)
- Preserve prefix style:
  - If user typed `../`, keep it
  - If user typed `~`, expand or not based on config:
    - If `expand_tilde` is false, preserve `~` and resolve internally only for listing
- Separator policy:
  - If `prefer_forward_slashes` true: insert `/` even on Windows unless user is clearly typing `\` paths and config says otherwise
  - Must be deterministic and documented

### 6.7 Completion triggering
- Server may provide `triggerCharacters` (e.g., `/`, `\`, `.`) but must not rely on them.
- Manual invocation (Ctrl-Space / Show Completions) must always work.

---

## 7) Windows & Cross-Platform Support (Phase 3)

Minimum Phase 3 expectations:
- Recognize drive prefixes:
  - `C:\` and `C:/`
- Recognize UNC prefixes when enabled:
  - `\\server\share\`
- Avoid generating broken Python escape sequences:
  - If inserting backslashes into non-raw strings, escape them or prefer forward slashes.
Recommended default:
- insert forward slashes for portability unless config requests OS-native.

Document Windows notes in README:
- how to type raw strings if they want backslashes
- how the server inserts separators by default

---

## 8) Performance & Reliability Requirements (Phase 3)

### 8.1 No recursion, no indexing
- List only one directory level per completion request.
- Never walk the entire workspace.

### 8.2 Cache behavior
- Cache directory listings with TTL and max size.
- Must be bounded and safe:
  - no unbounded memory growth
  - eviction policy (LRU or simple FIFO) is fine

### 8.3 Safety caps
- Always cap results.
- If a directory listing is huge, stop early.
- Avoid blocking for long:
  - If listing is slow, return partial results rather than hanging.

### 8.4 Error handling
- Missing directory → return empty list, not error
- Permission denied → return empty list, log once
- Broken symlink / IO error → skip entry and continue

---

## 9) Logging & Observability (Phase 3)

- Log only to stderr.
- Provide a debug mode (env var or flag) to increase verbosity.
- Never spam logs on every keystroke unless debug mode is enabled.
- Include one-line summaries for:
  - config loaded (once)
  - cache hits/misses (debug only)
  - why completion was gated off (debug only)

---

## 10) Testing Requirements (Phase 3)

### 10.1 Unit tests (must expand)
Add tests for:
- config parsing and defaulting
- context gating decisions (open/Path/read_csv examples)
- prefix fallback decisions
- Windows prefix parsing (if implemented)
- segment extraction boundaries and replacement ranges
- ignore patterns filtering (at least basic)

Tests must run via:
- `cargo test` in `server/`

### 10.2 Deterministic filesystem tests
Where filesystem is required:
- use temp directories created during tests
- create small synthetic trees
- avoid relying on developer machine contents

### 10.3 Manual test checklist (must update README)
Provide a Phase 3 checklist:
- examples where completions SHOULD appear (open, Path, pandas)
- examples where completions SHOULD NOT appear (random string)
- examples of prefix fallback (any string with `./`)
- Windows-specific (if available)

---

## 11) Documentation Requirements (Phase 3)

README must include:
1) What the extension does (short)
2) How to install as dev extension (unchanged)
3) How to build the server (unchanged)
4) Configuration reference:
   - every setting, meaning, default
   - example `settings.json` snippet showing `lsp.<server_id>.settings`
5) “When do completions appear?” explanation:
   - smart gating + prefix fallback
6) Troubleshooting:
   - logs
   - capability grants
   - server ordering
7) Known limitations:
   - be explicit (triple quotes, f-strings, etc.)

---

## 12) Git Hygiene (mandatory)

- `.gitignore` must ignore:
  - `/target/`
  - `/server/target/`
- Do not commit build artifacts.
- Keep commits focused and readable:
  - ideally one Phase 3 commit: `phase 3: context-aware + configurable path completions`
  - checkpoint commits allowed, but avoid noise

---

## 13) Security & Safety (Phase 3)

- No network calls.
- No downloading.
- No executing arbitrary commands beyond what the extension already needs to launch the local server.
- Do not broaden extension capabilities beyond what is already necessary.
- Never read outside the workspace unless the user explicitly types an absolute path (and config allows it).

---

## 14) Codex Process Checklist (what you must do each run)

Before edits:
- [ ] Summarize plan
- [ ] List files to touch
- [ ] Confirm no Phase 4 features
- [ ] Confirm completion-only advertising remains

During edits:
- [ ] Keep diffs minimal
- [ ] Add tests for new logic
- [ ] Update README if behavior/config changes

After edits:
- [ ] Run `cargo fmt` and `cargo test` in server/
- [ ] Summarize: what changed, why, commands run, limitations
- [ ] Ensure repo is clean: `git status` shows no target artifacts

---

## 15) Explicit Phase 3 Deliverables (file-by-file)

### `server/`
- Add/extend modules for:
  - configuration handling
  - context gating logic
  - improved string/segment extraction
  - Windows prefix support (if enabled)
- Expand tests (prefer multiple small test files).

### Extension (`src/lib.rs`, `extension.toml`)
- Ensure server launch remains stable.
- Ensure language server remains registered for Python.
- Do not add download logic.

### `.gitignore`
- Ensure build artifacts ignored (already done; verify).

### `README.md`
- Add configuration reference
- Add “when completions appear” explanation
- Update manual test plan and troubleshooting

---

## 16) What NOT to do (hard stops)

- Do not implement release/download pipeline.
- Do not implement non-completion LSP features.
- Do not add heavy parsing frameworks without explicit justification.
- Do not break Phase 2 basic functionality while improving gating/config.

---
