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

## Configuration (Phase 3)

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
        "show_hidden": false,
        "ignore_globs": [
          "**/.git/**",
          "**/.venv/**",
          "**/venv/**",
          "**/__pycache__/**",
          "**/.pytest_cache/**",
          "**/.mypy_cache/**",
          "**/.ruff_cache/**",
          "**/node_modules/**"
        ]
      }
    }
  }
}
```

### Settings reference

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

## Local Testing (Phase 3)

Build and run the server:

```sh
cd server
cargo build
cargo run
```

Install as a dev extension in Zed:

1. Open Zed and go to Extensions.
2. Choose Install Dev Extension.
3. Select this repo root (the folder containing `extension.toml`).

Minimal Python snippet to test:

```python
from pathlib import Path
import pandas as pd

open("./")
Path("./")
pd.read_csv("./")

print("hello")
print("./" )  # prefix fallback should still work
```

Notes:
- The server uses `/` as the inserted path separator for portability.
- The server binary path is expected at `server/target/debug/pathy-server`.
- If Zed prompts for permissions, grant `process:exec` to allow launching the server.

### Troubleshooting

- If completions don’t appear, trigger completion manually (e.g. Ctrl-Space).
- Check logs: Zed command palette → “Open Log”, or run `zed --foreground`.
- Verify your Python language server ordering includes `pathy` after the
  primary server.
- If the server won’t launch, confirm it was built and is executable.
- If the WASM build fails, install the `wasm32-wasip1` target and retry.

### Acceptance checklist (Phase 3)

- [ ] Completions appear in `open("...")`, `Path("...")`, and `read_csv("...")`.
- [ ] Completions do not appear in unrelated strings (e.g., `print("hello")`).
- [ ] Prefix fallback works anywhere inside a string with `./` or `../`.
- [ ] Selecting a completion replaces only the current path segment.
- [ ] Primary Python LSP features (e.g. go-to-definition) still work.
- [ ] No crash when the directory doesn’t exist.

## Known limitations

- Triple-quoted strings are only handled when the opening delimiter is on the
  same line as the cursor.
- F-strings are ignored when the cursor is inside `{...}` expressions.
- The server does not parse Python ASTs; gating is heuristic by design.
