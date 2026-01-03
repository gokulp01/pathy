# AGENTS.md — Codex Working Agreement (Phase 1: Repo Scaffold)

This repository is being built **from an empty repo** into a **Zed extension** that launches a **sidecar LSP server** to provide **filesystem path completions inside Python string literals**.

Codex: treat this file as the source of truth for how to work in this repo.

---

## 0) Project North Star

### What we are building
- A **Zed extension (Rust → WASM)** that:
  - provides a **secondary Python language server** (does not replace the user’s main Python LSP)
  - launches our server via `language_server_command` per Zed extension APIs
- A **native LSP server** (separate subproject) that:
  - implements `textDocument/completion`
  - offers **path completions** when the cursor is inside Python string literals (later phases add context awareness)

### Key constraint (do not violate)
- Zed extensions do **not** have VS Code–style hooks for arbitrary editor keystrokes or direct buffer edits.
- Therefore, **path intellisense must be delivered via LSP completions**, not editor macro hooks.

---

## 1) Phase 1 Scope (THIS PHASE ONLY)

### Goal of Phase 1
Create a clean, buildable, documented **skeleton** for:
1) Zed extension root (WASM crate + manifest)
2) LSP server subproject (empty-but-buildable)
3) Docs + license + dev install instructions
4) “Working agreements” so future Codex runs stay consistent

### Non-goals for Phase 1
- Do NOT implement any LSP completion logic yet.
- Do NOT add CI workflows yet.
- Do NOT add binary downloading / release automation yet.
- Do NOT add lots of dependencies “just in case.”

### Phase 1 Definition of Done
- Repo has a valid **Zed extension layout** with `extension.toml` at the repo root.
- Extension compiles (or at least has a standard build path documented) and is installable as a dev extension.
- `server/` exists as a separate project with a basic build target.
- README explains:
  - what the repo is
  - how to install as a dev extension
  - how to run/build the server project
- License file exists (MIT).
- Git is initialized and Phase 1 changes are committed.

---

## 2) Expected Repository Layout (after Phase 1)

Repo root:
- `extension.toml` (required by Zed)
- `Cargo.toml` + `src/lib.rs` (extension WASM crate)
- `server/` (separate project; preferred: Rust)
- `README.md`
- `LICENSE`
- `AGENTS.md` (this file)
- optional: `testdata/` (used in later phases)

Do NOT nest `extension.toml` under a subdirectory. Zed expects it at the extension root.

---

## 3) Codex Working Rules (follow every time)

### 3.1 Always start with a plan (before editing)
Before you change files, you must:
1) Summarize the task in 1–2 sentences
2) List the exact files you will create/modify
3) Describe the smallest-possible set of changes to reach the goal

### 3.2 Keep diffs small and scoped
- Prefer multiple small tasks over one huge task.
- Do not refactor unrelated code.
- Do not change formatting of unrelated files.

### 3.3 No surprise dependencies
- **Do not add new dependencies** unless:
  - they are required to build the skeleton, or
  - the user explicitly approves
- If you think a dependency is necessary, stop and explain:
  - why it’s needed
  - alternatives
  - impact on build complexity

### 3.4 Always validate
After changes:
- Run the most appropriate build/format checks.
- If a build command is uncertain, document the assumption clearly in README.
- Summarize results (success/failure) and next steps.

### 3.5 Be explicit about assumptions
If you assume anything (platform, toolchain availability, Zed behavior), say so clearly.

### 3.6 Do not leak secrets
- Never request or print API keys.
- Never embed tokens in repo files.
- If a command requires credentials, instruct the user to provide them via environment/config, not in source.

---

## 4) Tooling Standards

### Rust
- Prefer Rust 2021 edition unless Zed docs require otherwise.
- Avoid `unsafe` unless absolutely necessary (and then document why).
- Default to:
  - `cargo fmt`
  - `cargo clippy` (if configured)
  - `cargo test`
  - `cargo build`

### Filesystem + LSP server (future phases)
- Server should remain a separate subproject (do not mix server logic into WASM crate).
- WASM crate should stay thin: launching server + wiring config.

---

## 5) Zed Extension Guidelines (applies to all phases)

### Extension responsibilities
- Provide `extension.toml` metadata and entry points.
- Launch our sidecar LSP server using Zed extension APIs.
- Keep extension logic minimal and stable.

### Language server positioning
- The server must be designed to work as a **secondary** server.
- Avoid advertising capabilities beyond completion (later phases), to prevent interfering with primary Python LSP features.

---

## 6) Documentation Requirements (Phase 1 minimum)

### README must include
1) What this is
2) How to build the extension (WASM crate) — or at least how Zed loads it if build is handled by Zed
3) How to install as a dev extension in Zed (high-level steps)
4) How to build/run the `server/` subproject
5) Where configuration will live in later phases (brief mention only)

### LICENSE
- MIT license file must be present.

---

## 7) Git Workflow (mandatory)

### Checkpoints
Before any non-trivial change:
- Create a git checkpoint commit OR ensure a clean working tree.

After completing the task:
- Commit with a clear message (e.g., `phase 1: scaffold extension + server skeleton`)

### Commit message style
- Use `phase N: …` prefix when aligned with project phases.
- Keep messages descriptive and short.

---

## 8) How Codex Should Summarize Work (end of task)

At the end of each task, provide:
1) What changed (bullet list)
2) Why it changed (tie back to phase goals)
3) Commands run and results
4) Any follow-ups needed

---

## 9) Phase 1 Implementation Checklist (for Codex)

### Files to create (minimum)
- `extension.toml`
- `Cargo.toml`
- `src/lib.rs`
- `server/` (with its own project skeleton; prefer `server/Cargo.toml` and `server/src/main.rs`)
- `README.md`
- `LICENSE`
- (this) `AGENTS.md`

### extension.toml content guidance
- Include: id, name, version, schema_version, description, authors, repository (placeholder ok)
- Keep metadata consistent with README naming.

### Rust extension crate guidance
- Minimal compileable skeleton.
- Avoid heavy logic.
- If Zed requires specific crate types/build targets, follow Zed docs and document build steps in README.

### Server skeleton guidance
- Minimal “hello LSP” placeholder is fine later; for Phase 1 it can be an empty main that builds.
- No networking or downloads.

---

## 10) Safety / Platform Notes

- Assume users may be on macOS/Linux/Windows.
- Avoid shell scripts that only work on one platform unless clearly labeled.
- Prefer documenting commands in a cross-platform way (or give OS-specific variants).

---

## 11) What NOT to do in Phase 1 (hard stops)

- Do not implement actual completion behavior.
- Do not add CI workflows.
- Do not add binary download logic or require special Zed capabilities.
- Do not add large dependency stacks.
- Do not change repo structure away from standard Zed expectations.

---

## 12) Quick Reference: “Good” Phase 1 Output

✅ A clean scaffold that a human can open, understand, and build  
✅ Minimal metadata + docs in place  
✅ Clear separation between extension and server subproject  
✅ No premature complexity

---
