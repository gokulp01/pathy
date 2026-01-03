# AGENTS.md — Codex Working Agreement (Phase 4: Ship & Distribute)

This repository is a **Zed extension** (Rust → WASM) that launches a **sidecar LSP server** (native binary) providing **filesystem path completions inside Python string literals**.

Codex: treat this file as the source of truth for **Phase 4**. If any user prompt conflicts with this file, follow this file.

---

## 0) Phase 4 Objective (North Star)

### What “Phase 4” means
Phase 4 is about **shipping**:
- Users should be able to install the extension and have the sidecar LSP server available **without** manually building it.
- The extension must be able to **obtain the correct server binary for the user’s platform**, cache it, validate it, and run it.
- The project should have a **repeatable, secure-ish release process** producing binaries for supported platforms.

### Core constraints (still non-negotiable)
1) The sidecar remains **completion-only LSP** (no hover/definition/rename/etc).
2) The sidecar remains **secondary** to the user’s primary Python LSP.
3) No editor keystroke hooks; the feature remains delivered via **LSP completion**.
4) **Do not commit binaries** into git. Binaries are distributed via Releases.
5) Supply-chain safety: downloads must be **pinned to our release artifacts**, validated by checksum, and executed only after verification.

---

## 1) Scope: What Phase 4 MUST Deliver

### 1.1 Release automation (GitHub Actions)
- A CI workflow that runs on PRs:
  - format/lint/tests for server (and extension build sanity checks if applicable)
- A release workflow that:
  - builds server binaries for a platform matrix
  - packages them into consistent archives (tar.gz / zip)
  - produces checksums (sha256)
  - uploads artifacts to a GitHub Release (tag-driven or workflow-dispatch)

### 1.2 Extension runtime binary acquisition (auto-download + cache)
The extension must:
- detect the host platform/arch
- determine which release asset matches
- download a small manifest (or checksums file) and the asset
- verify sha256 checksum
- unpack to a cache directory
- mark executable when needed
- launch server using `process:exec`

### 1.3 Configuration & fallbacks
Users must be able to:
- disable auto-download and provide their own server path
- override server path for development (local build)
- optionally override release channel (stable vs prerelease) if we support it
- inspect/clear cache via documented manual steps (we can’t add arbitrary UI actions in Zed extensions)

### 1.4 Documentation
README must include:
- install as dev extension + published extension notes
- required capabilities and how to grant them
- configuration reference for Phase 4 settings
- troubleshooting for download failures, checksum failures, proxy/SSL issues
- release process for maintainers (tagging, workflows, versioning)

### 1.5 Publishing readiness
- Ensure licensing and metadata remain valid for Zed registry submission.
- Document the steps to publish to the Zed extension registry (PR to zed-industries/extensions).

---

## 2) Non-Goals (Hard Stops)

Do NOT do these in Phase 4:
- Implement new feature functionality (that was Phase 2/3)
- Add LSP capabilities beyond completion
- Add a recursive indexer or file watcher
- Add telemetry / analytics
- Add a custom UI panel in Zed (not supported by the current extension surface)
- Add network calls unrelated to downloading our official release assets
- Add automatic self-update beyond “download latest version matching this extension version”

---

## 3) Definition of Done (Phase 4)

Phase 4 is complete when:

### Release pipeline
- ✅ CI runs on PRs and passes
- ✅ Release workflow produces archives + sha256 checksums for each supported platform
- ✅ Release workflow uploads them to a GitHub Release
- ✅ Asset naming is stable and documented

### Extension behavior
- ✅ Fresh install on a supported platform:
  - downloads correct binary
  - verifies checksum
  - unpacks and caches
  - launches server
  - path completions work
- ✅ If download is disabled or fails:
  - extension falls back to a user-provided `server_path` (if configured)
  - otherwise produces a clear error message and docs point to fixes
- ✅ The extension never executes an unverified download
- ✅ Cache logic is deterministic and does not grow unbounded

### Docs & configuration
- ✅ README includes Phase 4 config + capabilities + troubleshooting
- ✅ Settings examples are copy/paste correct
- ✅ Known limitations are clearly documented

### Git hygiene
- ✅ No release binaries committed to the repo
- ✅ `.gitignore` covers any new build/dist folders

---

## 4) Supported Platforms (Phase 4)

Phase 4 MUST explicitly define and document supported platforms.

Minimum recommended set:
- macOS: x86_64 + arm64
- Linux: x86_64 (arm64 optional but nice)
- Windows: x86_64

Rules:
- If we do not support a platform, the extension must detect it and produce a clear error (“unsupported platform”) with remediation (build from source).

---

## 5) Versioning & Release Strategy (must be consistent)

### 5.1 Single source of truth for version
- Extension version in `extension.toml` is the canonical version.
- Server version should match extension version.
- Release tag MUST match that version (e.g., `v0.4.0` for version `0.4.0`).

### 5.2 Release artifacts tied to version
- Extension must download binaries for **its own version**, not “latest”.
- This prevents silent behavior changes and makes debugging reproducible.

### 5.3 Changelog
- Add/maintain a `CHANGELOG.md` or a release-notes section in README.
- Keep it short but accurate.

---

## 6) Release Artifact Naming (critical: must be deterministic)

Define a naming scheme and never change it casually.

Example scheme (illustrative; choose one and commit to it):
- Asset archive name:
  - `pathy-server_<VERSION>_<OS>_<ARCH>.tar.gz` for macOS/Linux
  - `pathy-server_<VERSION>_<OS>_<ARCH>.zip` for Windows
- Inside archive:
  - `pathy-server` (unix) or `pathy-server.exe` (windows)
- Checksums file:
  - `checksums-<VERSION>.txt` containing `sha256  filename`

Rules:
- Filenames must be ASCII, no spaces.
- OS strings should be one of: `macos`, `linux`, `windows`.
- ARCH strings should be one of: `x86_64`, `aarch64`.
- Document mapping logic in README.

---

## 7) Supply-Chain Safety Requirements (Phase 4 minimum bar)

We are not building a full secure update framework, but we must do the basics:

### 7.1 Download scope restriction
- Downloads must come only from our official GitHub Releases location.
- Do not download from arbitrary URLs.
- If a `base_url` override is supported, it must be clearly marked “advanced” and default to official repo.

### 7.2 Verify checksums
- Always verify sha256 checksum before unpacking/executing.
- If checksum mismatch:
  - delete the downloaded file
  - do NOT execute
  - surface clear error + troubleshooting hint

### 7.3 Avoid shell injection
- Never construct shell commands by concatenating untrusted strings.
- When executing processes, pass args as structured arguments.

### 7.4 Cache directory permissions
- Ensure the downloaded binary is stored in a directory writable by the user.
- Ensure executable bit is set on unix after extraction if needed.

---

## 8) Extension Download/Cache Design (must be explicit)

### 8.1 Cache location
- Choose a cache directory that is stable across runs.
- If Zed’s extension API provides a per-extension storage dir, use it.
- Otherwise use a safe OS-appropriate cache directory inside the user profile (document where it is).
- Cache directory structure must include version so multiple versions can coexist:
  - `<cache_root>/pathy/<version>/<platform>/pathy-server[.exe]`

### 8.2 Cache policy
- If binary exists and checksum is known-good → reuse it.
- If binary missing or checksum unknown → download and verify.
- Never re-download on every startup unless explicitly requested.

### 8.3 Offline mode
- If user is offline:
  - use cached binary if present
  - otherwise produce clear error and suggest manual server_path configuration

---

## 9) Configuration Requirements (Phase 4)

The server already has behavior settings (Phase 3). Phase 4 adds extension-level settings:

### 9.1 Extension settings (new)
Document these under a stable key (e.g., `lsp.pathy.settings` for server settings and `extensions.pathy` or similar for extension settings—use whichever mechanism the repo currently uses; do not invent unsupported config plumbing).

Required settings:
- `auto_download`: bool (default true)
- `server_path`: string | null (default null)
  - if set, use this binary and skip downloading
- `release_channel`: enum (default "stable")
  - stable only is acceptable; prerelease optional
- `base_url`: string | null (default null, advanced)
  - default uses official GitHub Releases
- `verify_checksum`: bool (default true; must remain true by default)
- `cache_dir`: string | null (default null; advanced)
  - if null, use default cache location

### 9.2 Precedence rules (must document)
When launching server:
1) If `server_path` is set and exists → use it
2) Else if `auto_download` true → download+cache+use
3) Else → fail with actionable error

---

## 10) GitHub Actions Requirements (Phase 4)

### 10.1 CI workflow (PR)
Must run:
- server: `cargo fmt` (check) + `cargo test`
- server: `cargo build --release` at least on one platform (or matrix if cheap)
- optionally: extension crate builds (sanity check) if feasible

Keep CI fast and reliable.

### 10.2 Release workflow
- Triggered by:
  - push tags matching `v*` OR manual workflow dispatch with version input
- Builds server binaries for the platform matrix.
- Packages archives with stable naming.
- Produces checksums.
- Creates or updates GitHub Release for that tag.
- Uploads:
  - all archives
  - checksums file(s)

Important:
- Do NOT require secrets beyond `GITHUB_TOKEN`.
- Avoid third-party actions that introduce risk unless widely used and necessary.

---

## 11) Testing Requirements (Phase 4)

### 11.1 Automated tests
- Unit tests for platform mapping, URL building, checksum parsing.
- Tests for cache path computation (pure functions).

### 11.2 Manual smoke tests (documented)
For each supported OS:
1) remove cache directory
2) install dev extension
3) grant required capabilities
4) open python file and verify completions
5) verify that binary was downloaded into cache
6) verify logs show “downloaded/verified/using cached” states

Provide exact steps in README.

### 11.3 Failure mode tests (manual)
Document how to simulate:
- checksum mismatch (e.g., corrupt download)
- missing network
- unsupported platform
and what the expected error looks like.

---

## 12) Documentation Requirements (Phase 4)

README must include:

### 12.1 User docs
- Installation (dev + registry)
- Required capabilities:
  - `process:exec` (to run server)
  - `download_file` (to download server)
  - How to grant them in Zed settings
- Configuration reference for:
  - server settings (Phase 3)
  - extension settings (Phase 4)
- Troubleshooting:
  - “download failed”
  - “checksum failed”
  - “server won’t launch”
  - “no completions”
  - proxy/SSL hints (generic)

### 12.2 Maintainer docs
- How to cut a release:
  1) bump versions
  2) update changelog
  3) tag `vX.Y.Z`
  4) verify release artifacts
- How to publish to Zed registry (PR to zed-industries/extensions)

---

## 13) Dependency & Code Rules (Phase 4)

### 13.1 Dependencies
- Keep new dependencies minimal.
- If checksum verification requires a crate, choose a small, well-known one.
- Prefer standard library when feasible.

### 13.2 No new runtime services
- No background daemons.
- No telemetry.

---

## 14) Git Hygiene (mandatory)

- Do not commit `dist/`, archives, or binaries.
- Ensure `.gitignore` covers:
  - `/dist/`, `/release/`, or any packaging output folder created by workflows/scripts
- Keep commits clean and focused.
Recommended commit message:
- `phase 4: release pipeline + auto-download server binaries`

---

## 15) Codex Process Checklist (must follow each run)

Before edits:
- [ ] Summarize plan
- [ ] List files to touch
- [ ] Confirm no feature changes / no LSP capability expansion
- [ ] Confirm artifacts naming scheme

During edits:
- [ ] Implement smallest viable pieces first:
  1) release workflow producing assets
  2) checksum generation/verification logic
  3) extension download+cache+launch logic
  4) docs updates

After edits:
- [ ] Run local tests where possible:
  - server tests
  - extension build sanity check if applicable
- [ ] Summarize changes + commands run
- [ ] Confirm no binaries committed
- [ ] Confirm `.gitignore` updated if needed

---
