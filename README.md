# Pathy (Zed Extension + Sidecar LSP)

Pathy is a Zed extension scaffold for a sidecar LSP server that will provide
filesystem path completions inside Python string literals. This Phase 1 repo
contains only the minimal extension and server skeletons.

## Extension (WASM crate)

The extension lives at the repo root as a minimal Rust `cdylib` crate.

Build (manual, if needed):

```sh
rustup target add wasm32-wasi
cargo build --target wasm32-wasi
```

Assumption: Zed can build the extension automatically when loading a dev
extension; confirm the exact target if your Zed setup requires a different
WASM target (some toolchains use `wasm32-wasip1` instead of `wasm32-wasi`).

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
