# EnvFish

> Your AI can use your secrets. Your AI never sees your secrets.

EnvFish is a **local-first** developer tool that keeps per-project environment
variables, secrets and (later) cloud-service connections on your machine, and
lets AI coding agents such as Claude Code or Codex *use* them through a broker
without ever *reading* them.

The MVP is complete end to end: projects, environments, PUBLIC/SECRET
variables, an encrypted vault (file or OS keychain), `.env` import/export, a
process runner, connections to external services, an AI permission engine with
human approvals, an audit log, a Secret Broker and an MCP server that Claude
Code / Codex can use — plus a CLI and a Tauri desktop app, both in Japanese and
English.

---

## 使い方 (Quick start, 日本語)

詳しい手順は [USAGE.md](USAGE.md) にあります。ここでは最短の流れだけ示します。

```bash
# セットアップ (Rust stable / Node 22 / pnpm 11)
pnpm install                                # Desktop アプリの依存関係
cargo install --path crates/cli --locked    # envfish コマンドを ~/.cargo/bin に入れる
envfish --version                           # 動作確認 (PATH に ~/.cargo/bin が必要)

# 1. 案件と環境を登録
envfish project add my-app --path ~/works/my-app
envfish use my-app
envfish env development --create

# 2. 変数を登録 (Secret は stdin から。argv には載せない)
envfish var set APP_URL http://localhost:3000
printf '%s' "$SUPABASE_SERVICE_KEY" | envfish var set-secret SUPABASE_KEY
envfish import .env            # 既存の .env をまとめて取り込む場合

# 3. 自分のアプリには環境変数として注入
envfish run pnpm dev

# 4. 外部サービスへの接続を定義 (認証情報は SECRET の「名前」で指す)
envfish connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY

# 5. Claude Code に MCP サーバーとして登録
claude mcp add envfish -- envfish mcp --client claude-code
```

これで Claude Code は `list_projects` / `list_connections` / `call_service` /
`supabase_select` などのツールを使えます。Secret の値を返すツールは存在せず、
Supabase などへの呼び出しは EnvFish が認証を付けて代行します。

| 状況 | 既定の判定 |
|---|---|
| development で READ (GET) | 許可 |
| development で WRITE (POST/PUT/PATCH) | 確認 → Desktop の「AI アクセス」か `envfish ai approve <id>` で承認 |
| DELETE、または production での WRITE 以上 | 拒否 |
| production で READ | 確認 |

判定は `envfish ai permit WRITE ALLOW --client claude-code --connection supabase` のように変更でき、
すべての結果が `envfish activity` と Desktop の「アクティビティ」に記録されます。

Desktop アプリは `pnpm dev` で起動します (Projects / Connections / AI Access / Activity / Settings。
日本語・英語、ライト・ダーク切替)。

---

## Repository layout

```text
envfish/
├ apps/
│  └ desktop/              Tauri 2 + React 19 + Vite 8 + Tailwind 4 (shadcn/ui-style)
│     ├ src/               pages, API layer (Zod-validated invoke wrappers)
│     └ src-tauri/         Rust shell: Tauri commands → envfish-core
├ crates/
│  ├ core/                 domain models, SQLx/SQLite repo, service façade `EnvFish`,
│  │                       permission engine, .env parser
│  ├ vault/                XChaCha20-Poly1305 sealing + `MasterKeyProvider` (file / OS keychain)
│  ├ broker/               Secret Broker: authenticated HTTP calls, credential scrubbed from responses
│  ├ mcp/                  MCP server (stdio, JSON-RPC 2.0) for Claude Code / Codex
│  ├ daemon/               Local Agent skeleton (request/response types, in-process handler)
│  └ cli/                  `envfish` binary (clap) + goldfish splash
├ packages/
│  └ ui/                   @envfish/ui — shared Button/Card/Input/Badge/Goldfish primitives
├ Cargo.toml               Cargo workspace (default-members = the four crates)
├ pnpm-workspace.yaml
└ README.md
```

## Getting started

Requirements: Rust stable (1.85+, edition 2024), Node 22, pnpm 11, and the
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for the desktop app.

```bash
pnpm install
cargo build          # builds core, vault, broker, mcp, daemon, cli
cargo test           # 44 tests across the Rust crates

# CLI
./target/debug/envfish project add my-app --path ~/works/my-app
./target/debug/envfish project list
./target/debug/envfish use my-app
./target/debug/envfish env development --create
./target/debug/envfish var set APP_URL http://localhost:3000
printf '%s' "$OPENAI_API_KEY" | ./target/debug/envfish var set-secret OPENAI_API_KEY
./target/debug/envfish var list          # secrets show as ••••••••
./target/debug/envfish status

# Desktop
pnpm dev             # tauri dev (Vite + Rust, hot reload)
pnpm desktop:build   # production bundle
```

Data lives in one directory (override with `ENVFISH_HOME`):

| macOS | `~/Library/Application Support/envfish/` |
|---|---|
| Linux | `~/.local/share/envfish/` |
| Windows | `%APPDATA%\envfish\` |

It holds `envfish.db` (SQLite), `master.key` (32 random bytes, mode `0600`)
and `state.json` (current project/environment ids for the CLI). Never commit it.

### CLI reference

| Command | Purpose |
|---|---|
| `envfish project list \| add <name> [--path p] \| remove <name>` | manage projects |
| `envfish use <project>` | select current project |
| `envfish env [<name> [--create]]` | list / select (create) an environment |
| `envfish var list \| set <NAME> <value> \| set-secret <NAME> \| remove <NAME>` | variables of the current project/environment |
| `envfish run <cmd...>` | run a command with the environment's variables **and decrypted secrets** injected into the child only |
| `envfish import [.env] [--yes] [--dry-run] [--no-gitignore] [--delete]` | import a `.env` with PUBLIC/SECRET classification (confirmed per variable on a TTY); the file is added to the repo's `.gitignore`; `--delete` removes it afterwards (asks first) |
| `envfish clean [dir] [--dry-run] [--yes]` | delete `.env*` files in the project folder whose variables are all stored in EnvFish; asks per file, never touches `.env.example` |
| `envfish export-example [.env.example]` | write a `.env.example` (secrets blank) |
| `envfish export-env [.env.local] [--force]` | write a real dotenv file (values included, mode 0600) for tools that only read files; refused if git tracks the path |
| `envfish connection list \| add <name> --kind k --url u --secret SECRET_NAME [--auth style] [--meta k=v]... \| remove <name>` | external service connections (`generic_http`, `openai`, `supabase`, `cloudflare`, `vercel`, `github`, `aws`); the credential is the *name* of a SECRET |
| `envfish cred list \| add <name> --kind account\|ssh\|database\|file [--field k=v \| k=@file \| k=-]... \| show \| set \| copy [--field f] \| remove` | structured credentials for humans: test accounts (with TOTP), SSH targets, databases, files/certificates |
| `envfish ssh <name> [-- args]` | SSH with a stored key (0600 temp file, removed on exit) |
| `envfish run --with-credentials <cmd>` | also expose credentials as `ENVFISH_CRED_<NAME>_<FIELD>` and file credentials as `ENVFISH_FILE_<NAME>` paths |
| `envfish scan [dir] [--all-environments]` | report files that contain a stored secret value (value never printed) and `.env` files tracked by git; exit 2 on findings |
| `envfish agent [--ping]` | run the Local Agent on a Unix socket (`<data dir>/agent.sock`, mode 0600) |
| `envfish ai clients \| register <name> [--kind k]` | AI clients (also auto-registered by `envfish mcp`) |
| `envfish ai check [--client c] [--connection c]` | effective READ / WRITE / DELETE decisions for the current scope |
| `envfish ai permit <ACTION> <ALLOW\|ASK\|DENY> [--client c] [--connection c] [--all-environments]` | set a rule |
| `envfish ai rules \| unpermit <id>` | list / delete rules |
| `envfish ai approvals [--all] \| approve <id> \| deny <id>` | human decisions for ASK requests |
| `envfish activity [--limit n]` | audit log |
| `envfish mcp --client <name>` | MCP server on stdio |
| `envfish config show \| language <ja\|en\|system> \| theme <system\|light\|dark>` | settings shared with the desktop app |
| `envfish vault status \| key-backend <file\|keychain>` | where the master key lives; migrate between file and OS keychain |
| `envfish status` | data dir, key location, counts, current selection |

Global flags: `--json` (machine-readable, no decoration), `--no-animation`, `-v`.
`envfish` alone prints the usage (with the splash on a TTY). Help and messages
are localized: `ENVFISH_LANG=ja` or a Japanese `LANG`.

### The goldfish

Milestone commands (`status`, `use`, `env <name>`, `project add`) open with three
pixel-art goldfish — red, *nishiki* (brocade: red / white / black with a gold
eye) and a black *demekin* (telescope goldfish with bulging eyes) — swimming left to right for roughly 0.8 s. Each terminal cell carries
two vertical pixels via half-block characters (`▀ ▄ █`), so the fish stay crisp
in any monospace font:

```text
  ▀▄    ▄██▀▀▄▄
   ▀█▄█████████▄     ~
   ▄█▀██████▀▀
  ▀     ▀▀
```

The same 11×8 sprite is rendered as SVG in the desktop app (`Goldfish` in
`@envfish/ui`), swims as the loading indicator (`GoldfishLoader` for page
loads, `GoldfishInline` inside pending buttons; both honour
`prefers-reduced-motion`), and drives the application icon.

The splash is suppressed whenever it could get in the way: stdout is not a TTY,
`CI` or `ENVFISH_NO_ANIMATION` is set, `TERM=dumb`, `--json`, or
`--no-animation`. `NO_COLOR` gives monochrome fish. The splash draws only inside
two lines it reserves for itself and restores the cursor, so command output
is never corrupted, and it has no access to vault data.

---

### Connection kinds

| kind | default base URL | auth |
|---|---|---|
| `generic_http` | required | `bearer` (or `header:<Name>`, `query:<name>`, `none`) |
| `openai` | `https://api.openai.com/v1` | `bearer` |
| `supabase` | required (`https://<ref>.supabase.co`) | `apikey` + `Authorization: Bearer` |
| `cloudflare` | `https://api.cloudflare.com/client/v4` | `bearer` |
| `vercel` | `https://api.vercel.com` | `bearer` |
| `github` | `https://api.github.com` | `bearer` |
| `aws` | required (service endpoint) | SigV4, signed in the Broker. Needs `--meta region=… --meta service=… --meta access_key_id_secret=<SECRET name>`; `--secret` is the secret access key |

```bash
printf '%s' "$AWS_ACCESS_KEY_ID"     | envfish var set-secret AWS_ACCESS_KEY_ID
printf '%s' "$AWS_SECRET_ACCESS_KEY" | envfish var set-secret AWS_SECRET_ACCESS_KEY
envfish connection add sqs --kind aws --url https://sqs.ap-northeast-1.amazonaws.com \
  --secret AWS_SECRET_ACCESS_KEY --meta region=ap-northeast-1 --meta service=sqs \
  --meta access_key_id_secret=AWS_ACCESS_KEY_ID
```

### Credentials (for humans)

Variables cover `NAME=value`. Credentials hold the messier things: a test account
(URL, username, password, TOTP seed), an SSH target (host, user, private key),
a database login, or a whole file such as a certificate or kubeconfig. Every
secret field is sealed per field; usernames count as secret too.

```bash
envfish cred add qa-admin --kind account --field url=https://staging.example.com/login   # prompts for username/password (hidden)
envfish cred add bastion  --kind ssh --field host=bastion.example.com --field user=deploy --field private_key=@~/.ssh/id_bastion
envfish cred add pg-stg   --kind database --field engine=postgres --field host=db.example.com --field database=app
envfish cred add ca-cert  --kind file --field filename=ca.pem --field content=@./ca.pem

envfish cred copy qa-admin --field password   # clipboard, cleared after 30 s (TTY only)
envfish cred copy qa-admin --field totp       # current one-time code
envfish ssh bastion -- uptime
envfish run --with-credentials -- psql        # ENVFISH_CRED_PG_STG_PASSWORD, ENVFISH_FILE_CA_CERT, ...
```

The desktop app has the same in its **Credentials** page, with copy buttons
that call into Rust: the value is placed on the clipboard without ever passing
through the webview. AI clients only get `list_credentials` (names, kinds and
non-secret fields such as host or URL).

## Using EnvFish from Claude Code

```bash
# 1. Store the credential and describe the service once
envfish use my-app && envfish env development
printf '%s' "$SUPABASE_SERVICE_KEY" | envfish var set-secret SUPABASE_KEY
envfish connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY

# 2. Register the MCP server with Claude Code (once per machine or project)
claude mcp add envfish -- envfish mcp --client claude-code
```

Claude Code now sees tools `list_projects`, `list_environments`, `list_connections`,
`list_variables` (PUBLIC values, SECRET names only), `call_service` and
`supabase_select`. When it calls Supabase, EnvFish evaluates the permission
engine: development READ is allowed, WRITE pauses until you approve it in the
desktop app's **AI Access** page or with `envfish ai approve <id>`, DELETE is
denied; production is stricter. Every decision lands in **Activity**. The
credential itself is injected by the Broker and scrubbed from responses; there
is no tool that returns it.

For Codex or another MCP client, point it at the same command with a different
`--client` name so permissions and the audit log stay per-client.

## Architecture

```text
   CLI (clap)          Desktop (Tauri + React)         [Phase 2: Local Agent / MCP]
        │                       │                               │
        └──────────┬────────────┘                               │
                   ▼                                            ▼
            envfish-core :: EnvFish  ◀─────────────── envfish-daemon :: Agent
              │ projects / environments / variables            (request set has
              │ list_variables → values null for SECRET          no GetSecret)
              ▼
     ┌────────────────┐        ┌──────────────────────────────────┐
     │ SQLite (SQLx)  │        │ envfish-vault                    │
     │  projects      │        │  Vault: XChaCha20-Poly1305 AEAD  │
     │  environments  │        │  MasterKeyProvider (trait)       │
     │  variables ──── plaintext (PUBLIC only)                    │
     │  secrets  ──── ciphertext + nonce ◀── seal/open ───────────┤
     └────────────────┘        │   ├ FileMasterKeyProvider (P1)   │
                               │   ├ InMemory (tests)             │
                               │   └ Keychain / DPAPI / Secret    │
                               │     Service (later phases)       │
                               └──────────────────────────────────┘
```

**core** owns all business rules. `EnvFish` is the single façade; the CLI and
the Tauri commands are thin adapters over it. Listing returns `Variable` whose
`value` is `None` for secrets, so the same type is safe for tables, JSON and
IPC.

**vault** knows nothing about projects. It seals a `SecretString` with a
random 24-byte nonce and binds it to associated data (the secret row id), so a
ciphertext cannot be re-attached to another row. Key storage is behind
`MasterKeyProvider`; swapping the file backend for an OS keychain touches no
other crate.

**broker** turns a `Connection` + request into an authenticated HTTP call.
The credential is resolved inside a closure (`EnvFish::with_secret`), injected
per `auth_style` (`bearer`, `header:<Name>`, `query:<name>`, `supabase`,
`none`) with the header marked sensitive, and any occurrence of it in the
response body or error text is replaced with `[REDACTED]`. Paths are confined to
the connection's host; caller-supplied auth headers are rejected.

**mcp** is a small hand-written JSON-RPC 2.0 stdio server. It registers the
client, evaluates `core::permission` for every brokered call, blocks on `ASK`
until a human resolves the approval row (Desktop / CLI) or it times out, and
writes an audit entry for every outcome. The tool list has no secret-returning
tool, and the test suite asserts that.

**daemon** is the Local Agent: `envfish agent` serves newline-delimited JSON
over a `0600` Unix domain socket (listing, approvals, audit, permission
decisions — no secret-returning request). The desktop app and MCP server still
link `envfish-core` directly today; the socket lets other local tools
coordinate without touching SQLite. Windows named pipes are not implemented.

**desktop** has Projects (environments, variables, `.env` import,
`.env.example` copy), Connections, AI Access (pending approvals, clients,
permission matrix, rules), Activity, an in-app Guide (bilingual, copyable commands) and Settings (language, theme, vault info).
Every IPC call goes through `src/lib/api.ts` and is validated with Zod, keeping
the command surface auditable. Language (日本語 / English / system) and theme
(light / dark / system) are stored in EnvFish settings, so the CLI and the app
agree; strings live in `src/lib/i18n.tsx`.

## Security decisions (Phase 1)

- **Secrets never reach SQLite in the clear.** They are stored in a separate
  `secrets` table that has no plaintext column, as XChaCha20-Poly1305
  ciphertext + nonce (RustCrypto `chacha20poly1305`; no home-grown crypto).
  Tests assert the plaintext is absent from every file in the data directory,
  including the WAL.
- **Separate, non-serializable secret types.** `SecretValue` (core) and
  `MasterKey` (vault) print `[REDACTED]` under `Debug`, have no `Display`,
  `Serialize`, `Clone` or `PartialEq`, and are zeroized on drop. Reading the
  content requires calling `expose()`, which is easy to grep for in review.
- **No AI-reachable "get secret".** Decryption is `pub(crate)`; the sanctioned
  consumers are `resolve_process_env` (for `envfish run`, human initiated), the
  closure-based `with_secret` used by the Broker, and `with_credential_field`
  behind the human-only copy / ssh / run paths. Neither
  the Tauri commands, the MCP tools nor `AgentRequest` can return a value. The
  desktop app cannot reveal a stored secret; a human-only reveal with OS
  authentication is still deliberately absent.
- **Permission decisions never read AI text.** The engine consults stored rules
  and environment names only. Defaults: development-like READ ALLOW / WRITE
  ASK / DELETE DENY; production-like READ ASK / WRITE DENY / DELETE DENY.
  `ASK` requires a human click or `envfish ai approve`, with a 3-minute timeout.
- **Broker hygiene.** Credentials are marked sensitive in headers, scrubbed
  from response bodies and error messages, requests cannot leave the
  connection host, redirects are not followed, bodies are capped at 256 KiB.
- **Plaintext crosses the webview boundary exactly once**, in
  `set_secret_variable`, where it is wrapped in `SecretValue` immediately and
  the incoming `String` is zeroized. The React side uses a password input,
  never caches the value, and never receives it back.
- **CLI never takes secrets from argv.** `var set-secret` reads stdin, with
  echo disabled on a TTY, so values do not land in shell history or `ps`.
- **Logs and errors carry identifiers only.** `CoreError` / `VaultError`
  variants embed names and paths, never values; AEAD failures are reported as
  one opaque `Decrypt` error.
- **Plaintext-emitting commands are human-only.** `envfish run`, `export-env`,
  `ssh` and `cred copy` require an interactive terminal and refuse to run inside
  known agent sessions (`CLAUDECODE`, Codex, Cursor, Gemini CLI markers). An AI
  with shell access therefore cannot call `envfish run env` to dump the vault;
  it gets the MCP broker instead. `ENVFISH_ALLOW_UNATTENDED=1` opts a script
  you run yourself back in. Metadata commands (`var list`, `status`, …) keep
  working for agents.
- **Master key is separate from the database.** Copying `envfish.db` alone
  yields nothing. New vaults on macOS default to the OS keychain
  (`ENVFISH_KEY_BACKEND=file` overrides, and existing `master.key` files are kept);
  elsewhere the default is a `0600` file and `envfish vault
  key-backend keychain` moves the key into the macOS Keychain / Windows
  Credential Manager / Linux Secret Service (read back before the file is
  deleted). Secrets need no re-encryption because the key bytes are unchanged.
- **Desktop capabilities are minimal.** Only `core:default`; no shell, fs or
  clipboard plugins. CSP is set.

## Not implemented yet

- Human-only secret reveal gated by Touch ID / Windows Hello (and the clipboard
  auto-clear that goes with it). Until the OS authentication is wired in, there
  is intentionally no reveal at all.
- AWS SSO / assumed-role credential flows (static access keys work via SigV4).
- Windows named-pipe transport for the Local Agent.
- Playwright end-to-end tests (Vitest unit tests exist).

## CI / releases

- `.github/workflows/ci.yml`: `cargo fmt` / `clippy -D warnings` / `cargo test`
  on Linux and macOS, plus `pnpm typecheck` / `pnpm test` / desktop build.
- `.github/workflows/release.yml`: on a `v*` tag, builds Tauri bundles
  (macOS arm64 / x86_64, Linux, Windows) and CLI tarballs into a draft release.

## Development notes

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

## 日本語サマリー

EnvFish は、案件ごとの環境変数・Secret・(将来的に) クラウド接続を **完全ローカル**
で管理し、Claude Code / Codex などの AI エージェントには Secret 本体を見せず、
Broker 経由で許可された操作だけを実行させるための開発者ツールです。

**実装済み (MVP v0.1)**

- Project / Environment / Variable (PUBLIC・SECRET) の管理
- Vault: XChaCha20-Poly1305 で暗号化し SQLite に保存。`secrets` テーブルには平文カラム自体がない
- Master Key と Vault の責務分離 (`MasterKeyProvider` trait。Phase 1 は `0600` のファイル、後続で OS Keychain)
- CLI `envfish`: `project list/add/remove`、`use`、`env`、`var list/set/set-secret/remove`、`status`。`--json`、`--no-animation`。ヘルプとメッセージは日本語対応 (`ENVFISH_LANG=ja` または `LANG`)。引数なしで usage を表示
- 使い方の手順書: [USAGE.md](USAGE.md)
- Desktop (Tauri 2 + React): Projects / Project Detail / Environments / Variables。Connections / AI Access / Activity はルートとナビだけ先行配置。**日本語・英語切り替え対応**
- `envfish run`: 復号した Secret を子プロセスの環境変数にだけ注入
- `.env` Import (PUBLIC / SECRET を自動分類し、TTY では 1 件ずつ確認) と `.env.example` Export
- Connection (generic_http / openai / supabase)。認証情報は SECRET 変数の「名前」で参照
- AI Permission Engine: クライアント × プロジェクト × 環境 × 接続 × 操作 (READ / WRITE / DELETE) で ALLOW / ASK / DENY。既定は development が READ 許可・WRITE 確認・DELETE 拒否、production が READ 確認・他は拒否
- ASK の承認フロー: Desktop の AI アクセス画面または `envfish ai approve` で人間が判断 (3 分でタイムアウト)
- Secret Broker: 認証ヘッダーを注入して外部 API を呼び、レスポンス中の認証情報を `[REDACTED]` に置換
- MCP Server (`envfish mcp`): Claude Code / Codex 向け。ツールは一覧系と `call_service` / `supabase_select` のみで、Secret を返すツールは存在しない
- 監査ログ (`envfish activity` / Desktop の Activity)
- マスターキーの OS Keychain 保存 (`envfish vault key-backend keychain`)
- 設定 (言語 ja / en / system、テーマ light / dark / system) を CLI と Desktop で共有
- ドット絵の金魚 (赤・錦・黒出目金) を CLI の節目コマンドで表示。非 TTY / CI / `--json` / `--no-animation` では出さず、`NO_COLOR` 対応
- Connector 追加: Cloudflare / Vercel / GitHub (Bearer)、AWS (SigV4 署名を Broker 内で実施。AWS 公式のテストベクタで検証)
- Credential (資格情報): テストアカウント (URL・ユーザー名・パスワード・TOTP)、SSH (ホスト・ユーザー・秘密鍵)、データベース、ファイル/証明書をフィールド単位で暗号化保存。`envfish cred copy` / Desktop のコピーボタンでクリップボードへ (30 秒で自動消去、値は画面に出さない)、`envfish ssh`、`envfish run --with-credentials`。AI には `list_credentials` で名前と非秘密フィールドのみ
- `envfish scan`: 作業ツリー内に Secret の値が漏れていないか、追跡中の `.env` が無いかを検査 (値は表示しない)
- `envfish agent`: Local Agent を Unix ソケット (0600) で起動
- GitHub Actions: CI (fmt / clippy / test / typecheck / vitest / build) とタグ時のリリースビルド
- Desktop でも同じドット金魚を使用。ヘッダー・空状態に加え、読み込み中や保存中は 2 匹が泳ぐローディング表示 (`GoldfishLoader` / `GoldfishInline`)。アプリアイコンも同じスプライト

**セキュリティ上の判断**

- Secret 型 (`SecretValue`, `MasterKey`) は `Debug` で `[REDACTED]`、`Display`/`Serialize`/`Clone` なし、drop 時に zeroize
- AI から到達しうる「Secret を返す API」を作らない。`EnvFish` の復号メソッドは `pub(crate)`、Agent の要求型にも該当バリアントなし
- 平文が webview から Rust に渡るのは `set_secret_variable` の 1 回のみ。受け取り直後に `SecretValue` へ移し、元バッファを zeroize
- CLI は Secret を argv から受け取らない (stdin、TTY ではエコー無効)
- ログ・エラーには識別子のみ。値は載らない

**Claude Code から使う**

```bash
claude mcp add envfish -- envfish mcp --client claude-code
```

**未実装**: Touch ID / Windows Hello 付きの手動 Reveal (OS 認証を組み込むまでは Reveal 自体を置かない方針)、AWS SSO / AssumeRole、Windows の名前付きパイプ、Playwright E2E。詳細は上記英語セクション参照。
