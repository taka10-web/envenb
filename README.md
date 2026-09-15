<div align="center">

# EnvEnb

**Your AI can use your secrets. Your AI never sees your secrets.**

Local-first environment variables, secrets and service credentials for AI-assisted development.

[![CI](https://github.com/taka10-web/envenb/actions/workflows/ci.yml/badge.svg)](https://github.com/taka10-web/envenb/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[日本語版](README.ja.md) · [使い方ガイド](USAGE.md) · [Architecture](docs/ARCHITECTURE.md)

</div>

---

Store a key once. Your app gets the real value; your coding agent gets a name and a
broker that calls the API on its behalf.

```console
$ envenb var list                        # what you see
my-app / development
NAME            KIND    VALUE
--------------  ------  ---------------------
APP_URL         PUBLIC  http://localhost:3000
OPENAI_API_KEY  SECRET  ••••••••

$ # what Claude Code sees through MCP
[
  { "name": "APP_URL",        "kind": "PUBLIC", "value": "http://localhost:3000" },
  { "name": "OPENAI_API_KEY", "kind": "SECRET", "value": null,
    "note": "value withheld; use call_service" }
]

$ # and if the agent reaches for the shell instead
error: envenb run: refused inside an AI agent session (CLAUDECODE).
Secrets are for humans and for `envenb run` started from a real terminal;
agents use the MCP broker.
```

## Why EnvEnb

- **Nothing leaves your machine.** SQLite plus an encrypted vault in your home
  directory. No account, no sync, no server.
- **Secrets are never returned by an API.** Not over MCP, not from the desktop app,
  not from the CLI. The broker makes the authenticated call and returns the response.
- **The agent still gets work done.** `call_service`, `supabase_select` and friends
  reach OpenAI, Supabase, GitHub, Cloudflare, Vercel or any HTTP API — and AWS with
  SigV4 signing.
- **You decide what it may do.** Per client, project, environment and connection:
  allow, ask or deny. `ask` pauses the agent until you approve.
- **Everything is logged.** Who called what, and whether it went through.
- **It replaces your `.env` files.** Import one, and EnvEnb adds it to `.gitignore`
  and offers to delete it.
- **Structured credentials too.** Test accounts with TOTP, SSH keys, database logins,
  certificates — copied to the clipboard for 30 seconds, never shown on screen.
- **CLI and desktop app**, sharing the same vault, in English and Japanese.

## Installation

Requires [Rust](https://rustup.rs) stable, Node.js 22 and pnpm 11.

```bash
git clone https://github.com/taka10-web/envenb && cd envenb
pnpm install
cargo install --path crates/cli --locked
envenb --version
```

<details>
<summary>If <code>envenb: command not found</code></summary>

`~/.cargo/bin` is not on your `PATH`. Add this to `~/.zshrc` and open a new terminal:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

</details>

The desktop app runs with `pnpm dev`, or `pnpm desktop:build` for a bundle.

## Quick start

```console
$ envenb project add my-app --path ~/works/my-app
Registered project my-app (0b0a…)

$ envenb env development --create
Project: my-app
Environment: development

$ envenb import .env.local
Import .env.local → my-app / development

NAME               KIND    VALUE
-----------------  ------  -----------
APP_URL            PUBLIC  http://localhost:3000
OPENAI_API_KEY     SECRET  ••••••••

Imported: PUBLIC 1 / SECRET 1
Added to .gitignore: .env.local

$ envenb run npm run dev
  EnvEnb · my-app / development · 1 public · 1 secrets injected
```

`run` takes any command — `npm run dev`, `python app.py`, `go run .`,
`docker compose up`, a shell script of your own. Values are injected into the
child process only. Nothing is written back to disk.

## Using EnvEnb with Claude Code

Describe the service once, then register the MCP server:

```bash
printf '%s' "$SUPABASE_SERVICE_KEY" | envenb var set-secret SUPABASE_KEY
envenb connection add supabase --kind supabase \
  --url https://xyz.supabase.co --secret SUPABASE_KEY

claude mcp add envenb -- envenb mcp --client claude-code
```

`--secret` takes the *name* of a stored secret, never its value.

Now ask Claude to read the table. It calls `supabase_select`; EnvEnb adds the key,
strips it from the response, and records the call:

```console
$ envenb activity
TIME            CLIENT       PROJECT/ENV         CONN      ACTION  REQUEST                  RESULT
09-14 12:30:02  claude-code  my-app/development  supabase  READ    GET /rest/v1/items       ALLOWED
09-14 12:31:15  claude-code  my-app/production   supabase  DELETE  DELETE /rest/v1/users    DENIED
```

Defaults, before you write a single rule:

| Environment | READ | WRITE | DELETE |
|---|---|---|---|
| development, staging | allow | ask | deny |
| production, prod, live | ask | deny | deny |

`ask` blocks the agent until you approve it in the desktop app or with
`envenb ai approve <id>`. Change any of it with `envenb ai permit`.

## Documentation

| | |
|---|---|
| [USAGE.md](USAGE.md) | Full guide: every command, the desktop app, credentials, troubleshooting (Japanese) |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | How the crates fit together |
| [docs/SECURITY-DESIGN.md](docs/SECURITY-DESIGN.md) | Why secrets are handled this way |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Building, testing, sending a change |
| `envenb --help` | Every command, in your terminal |

## Status

Working end to end: projects, environments, variables, credentials, the vault
(file or OS keychain), `.env` import and export, the process runner, connections,
the permission engine with approvals, the audit log, the broker, the MCP server,
a CLI and a desktop app.

**macOS only.** The vault leans on the macOS Keychain and the local agent on
Unix domain sockets, so macOS is the only platform that is built, tested and
shipped.

Not yet: AWS SSO and assumed roles, a Touch ID gated reveal, Playwright
end-to-end tests.

## Security

To report a vulnerability, see [SECURITY.md](SECURITY.md).

## License

MIT. See [LICENSE](LICENSE).
