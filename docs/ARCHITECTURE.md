# Architecture

```text
   CLI (clap)        Desktop (Tauri + React)        MCP server (stdio)
        │                     │                      Claude Code / Codex
        └──────────┬──────────┴──────────────┬──────────────┘
                   ▼                         ▼
            envenb-core :: EnvEnb    envenb-broker :: Broker
              projects / environments    injects the credential,
              variables / credentials    scrubs it from responses
              permission engine                   │
                   │                              │
                   ▼                              ▼
     ┌────────────────┐        ┌──────────────────────────────────┐
     │ SQLite (SQLx)  │        │ envenb-vault                    │
     │  projects      │        │  Vault: XChaCha20-Poly1305 AEAD  │
     │  environments  │        │  MasterKeyProvider (trait)       │
     │  variables ──── plaintext (PUBLIC only)                    │
     │  secrets  ──── ciphertext + nonce ◀── seal/open ───────────┤
     │  credentials   │        │   ├ FileMasterKeyProvider        │
     │  permissions   │        │   ├ KeychainMasterKeyProvider    │
     │  audit_logs    │        │   └ InMemory (tests)             │
     └────────────────┘        └──────────────────────────────────┘

   envenb-daemon :: Agent — optional Unix socket for other local tools.
   No request in its vocabulary returns a secret value.
```

**core** owns all business rules. `EnvEnb` is the single façade; the CLI and
the Tauri commands are thin adapters over it. Listing returns `Variable` whose
`value` is `None` for secrets, so the same type is safe for tables, JSON and
IPC.

**vault** knows nothing about projects. It seals a `SecretString` with a
random 24-byte nonce and binds it to associated data (the secret row id), so a
ciphertext cannot be re-attached to another row. Key storage is behind
`MasterKeyProvider`; swapping the file backend for an OS keychain touches no
other crate.

**broker** turns a `Connection` + request into an authenticated HTTP call.
The credential is resolved inside a closure (`EnvEnb::with_secret`), injected
per `auth_style` (`bearer`, `header:<Name>`, `query:<name>`, `supabase`,
`none`) with the header marked sensitive, and any occurrence of it in the
response body or error text is replaced with `[REDACTED]`. Paths are confined to
the connection's host; caller-supplied auth headers are rejected.

**mcp** is a small hand-written JSON-RPC 2.0 stdio server. It registers the
client, evaluates `core::permission` for every brokered call, blocks on `ASK`
until a human resolves the approval row (Desktop / CLI) or it times out, and
writes an audit entry for every outcome. The tool list has no secret-returning
tool, and the test suite asserts that.

**daemon** is the Local Agent: `envenb agent` serves newline-delimited JSON
over a `0600` Unix domain socket (listing, approvals, audit, permission
decisions — no secret-returning request). The desktop app and MCP server still
link `envenb-core` directly today; the socket lets other local tools
coordinate without touching SQLite. Windows named pipes are not implemented.

**desktop** has Projects (environments, variables, `.env` import,
`.env.example` copy), Connections, AI Access (pending approvals, clients,
permission matrix, rules), Activity, an in-app Guide (bilingual, copyable commands) and Settings (language, theme, vault info).
Every IPC call goes through `src/lib/api.ts` and is validated with Zod, keeping
the command surface auditable. Language (日本語 / English / system) and theme
(light / dark / system) are stored in EnvEnb settings, so the CLI and the app
agree; strings live in `src/lib/i18n.tsx`.

---

## Repository layout

```text
envenb/
├ apps/
│  └ desktop/              Tauri 2 + React 19 + Vite 8 + Tailwind 4 (shadcn/ui-style)
│     ├ src/               pages, API layer (Zod-validated invoke wrappers)
│     └ src-tauri/         Rust shell: Tauri commands → envenb-core
├ crates/
│  ├ core/                 domain models, SQLx/SQLite repo, service façade `EnvEnb`,
│  │                       permission engine, .env parser
│  ├ vault/                XChaCha20-Poly1305 sealing + `MasterKeyProvider` (file / OS keychain)
│  ├ broker/               Secret Broker: authenticated HTTP calls, credential scrubbed from responses
│  ├ mcp/                  MCP server (stdio, JSON-RPC 2.0) for Claude Code / Codex
│  ├ daemon/               Local Agent skeleton (request/response types, in-process handler)
│  └ cli/                  `envenb` binary (clap) + maiko splash
├ packages/
│  └ ui/                   @envenb/ui — shared Button/Card/Input/Badge/Maiko primitives
├ Cargo.toml               Cargo workspace (default-members = the four crates)
├ pnpm-workspace.yaml
└ README.md
```

---

See also: [SECURITY-DESIGN.md](SECURITY-DESIGN.md) for why secrets are handled this way,
and [../CONTRIBUTING.md](../CONTRIBUTING.md) for building and testing.
