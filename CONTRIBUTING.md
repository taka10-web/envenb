# Contributing

Thanks for taking a look. EnvFish is a Cargo workspace plus a pnpm workspace.

## Prerequisites

- Rust stable (1.85 or newer)
- Node.js 22 and pnpm 11
- The [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for the desktop app

```bash
pnpm install
cargo test          # Rust: core, vault, broker, mcp, daemon, cli
pnpm test           # Frontend: Vitest
pnpm dev            # Desktop app with hot reload
```

## Building and testing

- `cargo build` / `cargo test` cover the four crates. The Tauri crate is a
  workspace member but not a default member because it needs `apps/desktop/dist`
  (`pnpm --filter @envfish/desktop build`) before `cargo build -p envfish-desktop`.
- A **debug** build of the desktop binary loads the Vite dev server
  (`devUrl`, port 1420), not `dist/`. Running `target/debug/envfish-desktop`
  without Vite shows an empty window; use `pnpm dev`, which starts both.
  `ENVFISH_DEVTOOLS=1 pnpm dev` opens the WebKit inspector on launch.
- CLI help and messages follow `ENVFISH_LANG` (`ja` / `en`), falling back to
  `LC_ALL` / `LC_MESSAGES` / `LANG`. Running `envfish` with no arguments shows
  the splash and usage.
- A step-by-step guide in Japanese lives in [USAGE.md](USAGE.md).
- `cargo clippy --workspace --all-targets` and `cargo fmt --all -- --check` are clean.
- Frontend: `pnpm typecheck`, `pnpm build`.

---

## CI and releases

- `.github/workflows/ci.yml`: `cargo fmt` / `clippy -D warnings` / `cargo test`
  on Linux and macOS, plus `pnpm typecheck` / `pnpm test` / desktop build.
- `.github/workflows/release.yml`: on a `v*` tag, builds Tauri bundles
  (macOS arm64 / x86_64, Linux, Windows) and CLI tarballs into a draft release.

## Before opening a pull request

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --no-fail-fast
pnpm typecheck && pnpm test && pnpm --filter @envfish/desktop build
```

These are exactly what CI runs.

## Where things live

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the crate layout and how the
pieces fit together, and [docs/SECURITY-DESIGN.md](docs/SECURITY-DESIGN.md) for the
rules that secret handling must not break.

## Adding a feature that touches secrets

Two rules, both enforced by tests:

1. No API — CLI, Tauri command, MCP tool or agent request — may return a secret value.
   Decryption stays inside `envfish-core`, behind `with_secret` / `with_credential_field`.
2. Anything that puts plaintext in front of a person must go through the human gate
   in `crates/cli/src/human.rs`.
