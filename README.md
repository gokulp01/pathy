# Pathy (Zed Extension + Sidecar LSP)

Pathy is a Zed extension that launches a sidecar LSP server to provide
filesystem path completions inside Python string literals. The server is
completion-only and runs as a secondary Python language server.

## Extension (WASM crate)

The extension lives at the repo root as a minimal Rust `cdylib` crate.

Build (manual, if needed):

```sh
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1
```

Assumption: Zed can build the extension automatically when loading a dev
extension; confirm the exact target if your Zed setup requires a different
WASM target (older toolchains used `wasm32-wasi`).

### Install as a dev extension in Zed

1. Open Zed and go to Extensions.
2. Choose Install Dev Extension.
3. Select this repo root (the folder containing `extension.toml`).

## Server (sidecar LSP)

The server lives in `server/` as a standalone Rust project.

Build:

```sh
cargo build
```

Run:

```sh
cargo run
```

## Capabilities (Phase 4)

Pathy requires these extension capabilities:

- `process:exec` (launch the server)
- `download_file` (download release assets)

If your Zed config restricts capabilities, add these entries to
`granted_extension_capabilities`:

```json
{
  "granted_extension_capabilities": [
    { "kind": "process:exec", "command": "*", "args": ["**"] },
    { "kind": "download_file", "host": "github.com", "path": ["**"] }
  ]
}
```

## Configuration (Phase 4)

Configure the server via Zed settings. Keep your primary Python LSP first and
add `pathy` after it:

```json
{
  "languages": {
    "Python": {
      "language_servers": [
        "pyright",
        "pathy",
        "..."
      ]
    }
  },
  "lsp": {
    "pathy": {
      "settings": {
        "enable": true,
        "context_gating": "smart",
        "base_dir": "file_dir",
        "max_results": 80,
        "show_hidden": false
      }
    }
  }
}
```

### Extension settings (auto-download)

These settings live under `lsp.pathy.settings` and control binary download
behavior:

- `auto_download` (bool, default: true)
- `server_path` (string | null, default: null)
- `release_channel` ("stable" only for now)
- `base_url` (string | null, advanced)
- `verify_checksum` (bool, default: true)
- `cache_dir` (string | null, advanced; relative to the extension working dir)

Precedence:
1) `server_path` if set and exists
2) else `auto_download` if true
3) else error with instructions

Example:

```json
{
  "lsp": {
    "pathy": {
      "settings": {
        "auto_download": true,
        "verify_checksum": true,
        "server_path": null
      }
    }
  }
}
```

### Server settings (Phase 3)

Behavior / gating:
- `enable` (bool, default: true)
- `path_prefix_fallback` (bool, default: true)
- `context_gating` ("off" | "smart" | "strict", default: "smart")

Base directory:
- `base_dir` ("file_dir" | "workspace_root" | "both", default: "file_dir")
- `workspace_root_strategy` ("lsp_root_uri" | "disabled", default: "lsp_root_uri")

Listing and filtering:
- `max_results` (int, default: 80)
- `show_hidden` (bool, default: false)
- `include_files` (bool, default: true)
- `include_directories` (bool, default: true)
- `directory_trailing_slash` (bool, default: true)
- `ignore_globs` (array[string], default: common cache/venv/node_modules ignores)

Paths and separators:
- `prefer_forward_slashes` (bool, default: true)
- `expand_tilde` (bool, default: true)
- `windows_enable_drive_prefix` (bool, default: true)
- `windows_enable_unc` (bool, default: true)

Performance / cache:
- `cache_ttl_ms` (int, default: 500)
- `cache_max_dirs` (int, default: 64)
- `stat_strategy` ("none" | "lazy" | "eager", default: "lazy")

## When do completions appear?

Pathy uses two gates:

1) Smart context gating: completions appear inside strings for common path
   contexts like `open("...")`, `Path("...")`, and `pandas.read_csv("...")`.
2) Prefix fallback: if the user types a clear path prefix (`./`, `../`, `/`, `~`,
   or Windows drive/UNC), completions are allowed anywhere inside a string.

If neither gate matches, completions stay quiet to avoid noise.

## Release artifacts (Phase 4)

Asset naming scheme:
- `pathy-server_<VERSION>_<OS>_<ARCH>.tar.gz` (macOS/Linux)
- `pathy-server_<VERSION>_<OS>_<ARCH>.zip` (Windows)
- `checksums-<VERSION>.txt`

Supported OS/arch values:
- OS: `macos`, `linux`, `windows`
- ARCH: `x86_64`, `aarch64`

The extension downloads assets for its own version (not “latest”).

## Local Testing (Phase 4)

1) Remove cache directory (default: `cache/` under the extension working dir).
2) Install the dev extension.
3) Open a Python file and trigger completion inside `open("./")`.
4) Confirm the binary was downloaded and cached.
5) Restart Zed and confirm it reuses the cached binary.

## Troubleshooting

- Download fails: check network access and that GitHub is reachable.
- Checksum fails: delete the cached archive and retry; verify release assets.
- Offline: set `server_path` to a locally built binary.
- Server won’t launch: ensure `process:exec` capability is granted.
- No completions: confirm language server ordering includes `pathy` after primary.

## Release process (maintainers)

1) Bump version in `extension.toml` and `Cargo.toml`.
2) Update `CHANGELOG.md`.
3) Tag `vX.Y.Z` and push the tag.
4) Verify GitHub Release assets and checksums.

## Publishing to Zed registry

Open a PR against `zed-industries/extensions` adding this repository.

## Known limitations

- Triple-quoted strings are only handled when the opening delimiter is on the
  same line as the cursor.
- F-strings are ignored when the cursor is inside `{...}` expressions.
- The server does not parse Python ASTs; gating is heuristic by design.
