# Contributing

Thanks for taking a look. EnvEnb is a Cargo workspace plus a pnpm workspace.

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
  (`pnpm --filter @envenb/desktop build`) before `cargo build -p envenb-desktop`.
- A **debug** build of the desktop binary loads the Vite dev server
  (`devUrl`, port 1420), not `dist/`. Running `target/debug/envenb-desktop`
  without Vite shows an empty window; use `pnpm dev`, which starts both.
  `ENVENB_DEVTOOLS=1 pnpm dev` opens the WebKit inspector on launch.
- CLI help and messages follow `ENVENB_LANG` (`ja` / `en`), falling back to
  `LC_ALL` / `LC_MESSAGES` / `LANG`. Running `envenb` with no arguments shows
  the splash and usage.
- A step-by-step guide in Japanese lives in [USAGE.md](USAGE.md).
- `cargo clippy --workspace --all-targets` and `cargo fmt --all -- --check` are clean.
- Frontend: `pnpm typecheck`, `pnpm build`.

---

## CI and releases

- `.github/workflows/ci.yml`: `cargo fmt` / `clippy -D warnings` / `cargo test`
  on macOS, plus `pnpm typecheck` / `pnpm test` / desktop build.
- `.github/workflows/release.yml`: on a `v*` tag, builds Tauri bundles
  (macOS arm64 / x86_64) and CLI tarballs into a draft release.

## Where things live

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the crate layout and how the
pieces fit together, and [docs/SECURITY-DESIGN.md](docs/SECURITY-DESIGN.md) for the
rules that secret handling must not break.

## Adding a feature that touches secrets

Four rules, all enforced by tests:

1. No API — CLI, Tauri command, MCP tool or agent request — may return a secret value.
   Decryption stays inside `envenb-core`, behind `with_secret` / `with_credential_field`.
2. Nothing hands a secret to a child process, an application or an AI agent. A value
   in `process.env` can be read back by any code in that process, so applications
   reach providers through the proxy instead.
3. Anything that puts plaintext in front of a *person* — `export-env`, `ssh`,
   `cred copy`, `var copy` — goes through the human gate in
   `crates/cli/src/human.rs`. `envenb run` does not, because it emits no plaintext.
4. A migration under `crates/core/migrations/` that has shipped is never edited,
   not even a comment. SQLx records a checksum, so any change locks every existing
   vault out. Schema changes go in a new numbered file.

## Issues

Use the templates. They ask for `envenb status`, a version and macOS version
because most reports are unreproducible without them.

**Never paste a real credential** — not in a log, a command or a screenshot.
Replace it with `sk-EXAMPLE`.

**A vulnerability is not an issue.** Report it privately through
[Security Advisories](https://github.com/taka10-web/envenb/security/advisories/new)
so a fix can ship before the details are public.

## Pull requests

- One change per pull request. A refactor bundled with a fix is hard to review
  and harder to revert.
- Say how you verified it, not just that tests pass. If something cannot be
  tested automatically, say why.
- Update the docs in the same pull request when behaviour changes: `README.md`,
  `README.ja.md`, `USAGE.md`, the in-app guide, and `docs/` where relevant.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org).
  Breaking changes take a `!` (`feat!: …`) and explain the migration in the body.
- Run the same checks CI does before opening it:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --no-fail-fast
pnpm typecheck && pnpm test && pnpm --filter @envenb/desktop build
```
