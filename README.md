# Pathy (Zed Extension + Sidecar LSP)

Pathy is a Zed extension scaffold for a sidecar LSP server that will provide
filesystem path completions inside Python string literals. This Phase 1 repo
contains only the minimal extension and server skeletons.

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

## Server (sidecar LSP skeleton)

The server lives in `server/` as a standalone Rust project.

Build:

```sh
cargo build
```

Run (placeholder):

```sh
cargo run
```

## Configuration (future)

Configuration will live in Zed extension settings in later phases.

## Local Testing (Phase 2)

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

Add the sidecar server as a secondary Python language server (keep your
primary server first):

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
  }
}
```

Minimal Python snippet to test:

```python
open("./")
open("../")
open("~/")
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

### Acceptance checklist

- [ ] Completions appear inside Python string literals for path-ish prefixes.
- [ ] Selecting a completion replaces only the current path segment.
- [ ] Primary Python LSP features (e.g. go-to-definition) still work.
- [ ] No crash when the directory doesn’t exist.
