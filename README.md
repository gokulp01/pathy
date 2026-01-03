# Pathy

Pathy is a Zed extension (Rust -> WASM) that launches a sidecar LSP server to
provide filesystem path completions inside Python string literals. It runs as a
secondary language server and only advertises completion capability.


<p align="center">
  <video width="600" controls autoplay loop muted>
    <source src="/static/pathy-sm.mp4" type="video/mp4">
    Your browser does not support the video tag.
  </video>
</p>


## Install (dev)

1) Build the extension (Zed builds WASM automatically) or open the repo in Zed.
2) In Zed, open the command palette and select "Extensions: Install Dev Extension".
3) Choose this repo root (the directory containing `extension.toml`).
4) Grant required capabilities if prompted (see below).

## Required capabilities

The extension needs:
- `process:exec` to launch the sidecar server
- `download_file` to fetch release binaries

These are already declared in `extension.toml`. If Zed prompts for approval,
accept them.

## Configuration

Settings live under `lsp.pathy.settings` in your Zed settings. This single block
contains both extension-level settings (download behavior) and server settings
(completion behavior).

### Language server ordering (Python)

Ensure your primary Python LSP comes first, then Pathy, then the fallback:

```json
{
  "languages": {
    "Python": {
      "language_servers": ["pyright", "pathy", "..."]
    }
  }
}
```

### Extension settings (download/runtime)

```json
{
  "lsp": {
    "pathy": {
      "settings": {
        "auto_download": true,
        "server_path": null,
        "release_channel": "stable",
        "base_url": null,
        "verify_checksum": true,
        "cache_dir": null
      }
    }
  }
}
```

Notes:
- `server_path` takes precedence. If set and exists, no download is attempted.
- `auto_download` must be `true` to fetch release assets.
- `release_channel` only supports `stable`.
- `base_url` is advanced; default points at `https://github.com/gokulp01/pathy`.
- `cache_dir` must be a relative path (relative to the extension working dir).

### Server settings (completion behavior)

Defaults shown in parentheses:

- `enable` (true)
- `path_prefix_fallback` (true)
- `context_gating` ("smart"): "off" | "smart" | "strict"
- `base_dir` ("file_dir"): "file_dir" | "workspace_root" | "both"
- `workspace_root_strategy` ("lsp_root_uri"): "lsp_root_uri" | "disabled"
- `max_results` (80)
- `show_hidden` (false)
- `include_files` (true)
- `include_directories` (true)
- `directory_trailing_slash` (true)
- `ignore_globs` (["**/.git/**", "**/.venv/**", "**/venv/**", "**/__pycache__/**",
  "**/.pytest_cache/**", "**/.mypy_cache/**", "**/.ruff_cache/**", "**/node_modules/**"])
- `prefer_forward_slashes` (true)
- `expand_tilde` (true)
- `windows_enable_drive_prefix` (true)
- `windows_enable_unc` (true)
- `cache_ttl_ms` (500)
- `cache_max_dirs` (64)
- `stat_strategy` ("lazy"): "none" | "lazy" | "eager"

Example override:

```json
{
  "lsp": {
    "pathy": {
      "settings": {
        "max_results": 40,
        "show_hidden": true,
        "context_gating": "strict"
      }
    }
  }
}
```

## Local testing (Phase 4)

1) Delete the cache directory to force a fresh download:
   - Default cache root: `cache/pathy/<version>/<os>/<arch>/`
2) Install as a dev extension (see above).
3) Open a Python file and try:
   - `open("./")`
   - `open("../")`
   - `open("~/")` (if `expand_tilde` is true)

To use a local build of the server:

```bash
cd server
cargo build --release
```

Then set `server_path` to the built binary:
- macOS/Linux: `server/target/release/pathy-server`
- Windows: `server/target/release/pathy-server.exe`

## Troubleshooting

- Download failed / 404: Confirm the GitHub Release for your version contains:
  - `checksums-<version>.txt`
  - `pathy-server_<version>_<os>_<arch>[.exe]`
- Checksum failed: Delete the cache directory and retry.
- Unsupported platform: Only macOS (x86_64/aarch64), Linux (x86_64), Windows (x86_64).
- No completions: Confirm Pathy is in `languages.Python.language_servers` and the
  cursor is inside a Python string.

Logs:
- Zed: open the log window.
- CLI: run `zed --foreground` to see extension logs.

## Release assets

Assets are raw binaries (no archives):
- `pathy-server_<VERSION>_macos_x86_64`
- `pathy-server_<VERSION>_macos_aarch64`
- `pathy-server_<VERSION>_linux_x86_64`
- `pathy-server_<VERSION>_windows_x86_64.exe`
- `checksums-<VERSION>.txt`

## Release process (maintainers)

1) Bump `version` in `extension.toml` and `Cargo.toml`.
2) Update `CHANGELOG.md`.
3) Tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.
4) Verify release assets and checksums on GitHub.

## License

MIT. See `LICENSE`.
