# EnvEnb 使い方ガイド

案件ごとの環境変数・Secret・認証情報を **完全ローカル** で管理し、Claude Code などの
AI エージェントには中身を見せずに使わせるためのツールです。

> AI に Secret を渡すのではなく、Secret を使う機能だけを渡す。

**目次**

- [5 分で始める](#5-分で始める) — インストールから最初の起動まで
- [やりたいことから引く](#やりたいことから引く) — 日々の操作
- [AI に使わせる](#ai-に使わせる) — Claude Code / Codex 連携
- [Desktop アプリ](#desktop-アプリ)
- [コマンド一覧](#コマンド一覧)
- [仕組みと安全性](#仕組みと安全性) — なぜこの形なのか
- [困ったとき](#困ったとき)

---

## 5 分で始める

### 1. インストール

**EnvEnb は macOS 専用です。** Vault が macOS Keychain を、Local Agent が
Unix ドメインソケットを使うためです。

必要なもの: [Rust](https://rustup.rs) stable (1.85 以上)。Desktop アプリも使う
場合は Node.js 22 以上、pnpm 11 以上、Xcode Command Line Tools。

**CLI**

```bash
git clone https://github.com/taka10-web/envenb && cd envenb
cargo install --path crates/cli --locked    # envenb コマンドを ~/.cargo/bin へ
envenb --version
```

`command not found` になる場合は `~/.cargo/bin` が PATH にありません。`~/.zshrc` に
`export PATH="$HOME/.cargo/bin:$PATH"` を追記し、新しいターミナルを開いてください。
更新するときは同じコマンドに `--force` を付けます。

**Desktop アプリ**

```bash
pnpm install
pnpm desktop:build
```

ビルドの最後に、生成された `release/bundle/macos/EnvEnb.app` のパスが表示されます。
これを `/Applications` にドラッグすれば、以後は通常のアプリとして起動できます。

CLI と Desktop アプリは同じ Vault を読み書きするので、どちらを使っても構いません。

### 2. 案件と環境を用意する

EnvEnb は **プロジェクト → 環境 → 変数** の 3 階層です。
まずプロジェクトと環境を選ばないと、変数の操作はできません。

```bash
envenb project add my-app --path ~/works/my-app   # 登録 (最初の 1 件は自動で選択されます)
envenb use my-app                                 # 以降の操作対象にする
envenb env development --create                   # 環境を作って選択
envenb status                                     # 今どこを触っているかの確認
```

### 3. 既存の .env を取り込む

ファイル名は何でも構いません。引数を省くと `.env` を見ます。

```bash
envenb import                    # .env
envenb import .env.local
envenb import .env.production
```

変数ごとに PUBLIC (AI に見えてよい設定値) か SECRET (鍵・トークン) かを聞かれます。
Enter を押すと提案どおりです。取り込んだファイルは自動で `.gitignore` に追記されます。

#### .env ファイルの種類について

慣習として使い分けられているだけで、**中身の形式はどれも同じ** (`KEY=value`) です。
EnvEnb は名前で扱いを変えます。

| ファイル | 中身 | git | EnvEnb の扱い |
|---|---|---|---|
| `.env` | 実際の値 | 入れない | 取り込む・削除対象 |
| `.env.local` | 実際の値 (自分の環境だけ) | 入れない | 取り込む・削除対象 |
| `.env.production` など | 環境ごとの実際の値 | 入れない | 取り込む・削除対象 |
| `.env.example` | **値が空の見本** | **入れる** | 触らない |

`.env.example` はチームに「この変数が要る」と伝えるためのテンプレートで、
値が入っていないので git に入れて共有します。**EnvEnb はこれを消しません。**
`.env.sample` `.env.template` も同じ扱いです。

EnvEnb から見本を書き出すこともできます。

```bash
envenb export-example .env.example   # PUBLIC は値付き、SECRET は名前だけ
```

### 4. アプリを起動する

```bash
envenb run npm run dev
envenb run python app.py
envenb run go run .
envenb run docker compose up
```

`run` の後ろには **任意のコマンド** が書けます。特定の言語やパッケージ
マネージャに依存しません。値は **子プロセスの環境変数にだけ** 渡ります。`.env.local` はもう不要なので削除できます。

```bash
envenb import .env.local --delete   # 取り込みと同時に (確認あり)
envenb clean                        # あとからまとめて
```

ファイル内のすべての変数が保存済みのときだけ削除され、`.env.example` には触れません。

---

## やりたいことから引く

### 変数を追加する

PUBLIC は引数で、SECRET は **標準入力** で渡します (シェル履歴や `ps` に残さないため)。

```bash
envenb var set APP_URL http://localhost:3000     # PUBLIC
envenb var set-secret OPENAI_API_KEY             # 対話入力、画面には出ません
op read "op://Dev/OpenAI/credential" | envenb var set-secret OPENAI_API_KEY   # パイプでも可
```

変数名は `[A-Za-z_][A-Za-z0-9_]*` です。

### 種別をあとから変える

```bash
envenb var kind API_TOKEN SECRET    # PUBLIC の値をそのまま暗号化
```

Desktop では一覧の `PUBLIC` バッジを押すと同じことができます。

**SECRET → PUBLIC はできません。** 暗号化した値を AI が読める平文の列に戻すことになるためです。
秘密でなかった場合は `envenb var remove` してから入れ直してください。

### 中身を確認する

```bash
envenb var list
```

```text
my-app / development
NAME            KIND    VALUE
--------------  ------  ---------------------
APP_URL         PUBLIC  http://localhost:3000
OPENAI_API_KEY  SECRET  ••••••••
```

SECRET の値を**画面に表示する**コマンドはありません。これは仕様です
([仕組みと安全性](#仕組みと安全性))。

どうしても値そのものが必要なときは、クリップボード経由で取り出せます。

```bash
envenb var copy OPENAI_API_KEY
```

画面にもスクロールバックにも残さず、30 秒後にクリップボードを自動で消去します
(その間に別のものをコピーした場合は消しません)。対話端末からのみ実行でき、
AI エージェントのセッション内では拒否されます。

ツールに `.env` を渡す必要がある場合は `envenb export-env` を使ってください
(`0600` で作成し、git 追跡下のパスには書きません)。

### 環境を切り替える

```bash
envenb env                        # 一覧
envenb env staging --create       # 作って選択
envenb env production             # 切り替え
```

`envenb use` でプロジェクトを変えると、環境の選択は解除されます。

### 環境ごとに値を出し分ける

同じ変数名を環境ごとに登録しておけば、`envenb env` を切り替えるだけで
`envenb run` に渡る値が変わります。

```bash
envenb env production
envenb var set-secret OPENAI_API_KEY   # 本番用の値
```

### `.env.example` を作る / 実値のファイルが必要なとき

```bash
envenb export-example                 # SECRET は空欄のテンプレート (コミット可)
envenb export-env .env.local          # 実値入り。0600 で作成し .gitignore に追記
```

`export-env` は、ファイルしか読めないツールのための逃げ道です。平文が作業ディレクトリに
戻るので、常用はしないでください。git が追跡中のパスには書き込みません。

### テストアカウント・SSH・DB・証明書を保存する

`NAME=値` に収まらないものは **Credential** として保存します。

| 種別 | フィールド (`*` は必須、**太字**は暗号化) |
|---|---|
| `account` | url, **username***, **password***, **totp_secret** |
| `ssh` | host*, port, user*, **private_key**, **passphrase**, **password** |
| `database` | engine, host*, port, database, **username***, **password*** |
| `file` | filename*, **content*** |

```bash
envenb cred add qa-admin --kind account --field url=https://staging.example.com/login
#   username / password / totp_secret は非表示で対話入力されます
envenb cred add bastion --kind ssh --field host=bastion.example.com --field user=deploy \
  --field private_key=@~/.ssh/id_bastion
envenb cred add ca-cert --kind file --field filename=ca.pem --field content=@./ca.pem
envenb cred list
```

使うときは、値を画面に出さずに渡します。

```bash
envenb cred copy qa-admin --field password   # クリップボードへ。30 秒後に自動消去
envenb cred copy qa-admin --field totp       # 現在のワンタイムコード
envenb ssh bastion -- uptime                 # 鍵は 0600 の一時ファイル、終了時に削除
```

### 漏えいを検査する

```bash
envenb scan                       # カレントディレクトリ
envenb scan ~/works/my-app --all-environments
```

保存済み Secret の値がファイルに現れていないか、`.env` が git に追跡されていないかを調べます。
値は表示せず `ファイル:行 変数名` だけを出し、見つかると終了コード 2 を返すので
pre-commit フックや CI に組み込めます。

### マスターキーを OS のキーチェーンに移す

```bash
envenb vault status
envenb vault key-backend keychain   # macOS Keychain へ移す
```

鍵を書き込んで読み戻せることを確認してから元を削除します。Secret の再暗号化は不要です。

### 表示を変える

```bash
envenb config language ja      # ja | en | system
envenb config theme dark       # light | dark | system (Desktop と共通)
```

舞妓のアニメーションは `--no-animation`、`ENVENB_NO_ANIMATION=1`、`NO_COLOR=1` で抑えられます。
CI・パイプ・`--json` のときは自動で出ません。

### スクリプトから使う

```bash
envenb project list --json | jq '.[].name'
envenb var list --json | jq '.[] | select(.kind == "SECRET") | .name'
```

`--json` でも SECRET の `value` は常に `null` です。

---

## AI に使わせる

### 1. 接続を定義する

認証情報は先に SECRET として登録し、接続には **その名前** を渡します (値は渡しません)。

```bash
printf '%s' "$SUPABASE_SERVICE_KEY" | envenb var set-secret SUPABASE_KEY
envenb connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY
envenb connection add openai --kind openai --secret OPENAI_API_KEY      # URL は既定値
envenb connection list
```

| kind | 既定 URL | 認証 |
|---|---|---|
| `generic_http` | 必須 | `--auth bearer` (既定) / `header:<名前>` / `query:<名前>` / `none` |
| `openai` | `https://api.openai.com/v1` | bearer |
| `supabase` | 必須 | apikey + Bearer |
| `cloudflare` | `https://api.cloudflare.com/client/v4` | bearer |
| `vercel` | `https://api.vercel.com` | bearer |
| `github` | `https://api.github.com` | bearer |
| `aws` | 必須 (サービスのエンドポイント) | SigV4 |

AWS は署名に追加情報が要ります。

```bash
envenb connection add sqs --kind aws --url https://sqs.ap-northeast-1.amazonaws.com \
  --secret AWS_SECRET_ACCESS_KEY --meta region=ap-northeast-1 --meta service=sqs \
  --meta access_key_id_secret=AWS_ACCESS_KEY_ID
```

### 2. Claude Code に登録する

```bash
claude mcp add envenb -- envenb mcp --client claude-code
```

Codex など別のツールは `--client codex` のように名前を変えると、権限と監査ログが分かれます。

AI に見えるツールは次の 6 つです。**Secret を返すものはありません。**

一覧系のツールはオブジェクトを返します (`{"projects": [...]}` のように)。MCP の仕様で
`structuredContent` はオブジェクトである必要があるためです。プロジェクトが 1 つだけのときは
`project` 引数を省略できます。

| ツール | 内容 |
|---|---|
| `list_projects` / `list_environments` / `list_connections` | 一覧 (認証情報は名前と状態のみ) |
| `list_variables` | PUBLIC は値付き、SECRET は名前のみ |
| `list_credentials` | 名前と非秘密フィールド (host, url など) のみ |
| `call_service` | 接続経由で HTTP を実行。EnvEnb が認証を付け、レスポンスから認証情報を除去して返す |
| `supabase_select` | `GET /rest/v1/<table>` の簡易版 |

### 3. 許す操作を決める

判定は クライアント × プロジェクト × 環境 × 接続 × 操作 の組み合わせごとです。
ルールが無いときの既定は次のとおりです。

| 環境 | READ | WRITE | DELETE |
|---|---|---|---|
| development など | ALLOW | ASK | DENY |
| production / prod / live | ASK | DENY | DENY |

```bash
envenb ai check --client claude-code --connection supabase        # 実効判定を確認
envenb ai permit WRITE ALLOW --client claude-code --connection supabase
envenb ai rules                                                   # 明示ルールの一覧
```

ASK になった要求は人間が判断します (3 分で期限切れ)。

```bash
envenb ai approvals          # 保留中の一覧
envenb ai approve <ID先頭8桁>
envenb activity              # 監査ログ: 誰が・どこに・何を・結果
```

Desktop の「AI アクセス」画面でも同じ承認ができ、2 秒ごとに更新されます。

---

## Desktop アプリ

```bash
pnpm dev             # 開発起動 (Vite + Tauri)
pnpm desktop:build   # 配布用バンドル
```

> debug ビルドのバイナリは Vite 開発サーバー (port 1420) を読むため、単体起動では白い画面になります。
> 必ず `pnpm dev` を使ってください。インスペクタは `ENVENB_DEVTOOLS=1 pnpm dev` で開きます。

CLI と同じデータを読み書きするので、どちらで登録しても双方に反映されます。

| 画面 | できること |
|---|---|
| 変数 | PUBLIC / SECRET の追加・削除、`.env` の取り込み (ドラッグ & ドロップ可)、`.env.example` のコピー |
| 資格情報 | テストアカウント / SSH / DB / ファイルの登録、フィールド単位のコピー、TOTP コード |
| 接続 | 環境ごとの接続の一覧と追加 |
| AI アクセス | 承認待ちの処理、クライアント登録、権限マトリクス、ルール一覧 |
| アクティビティ | 監査ログ |
| プロジェクト | プロジェクトと環境の管理 |
| 使い方 | 手順の要約 (コマンドはコピー可) |
| 設定 | 言語、テーマ、Vault の情報 |

上部のプロジェクト / 環境の切り替えが、全画面の対象を決めます。

---

## コマンド一覧

引数なしの `envenb`、`envenb --help`、`envenb <コマンド> --help` でも確認できます。

| コマンド | 内容 |
|---|---|
| `project add/list/remove` | プロジェクトの管理 |
| `use <名前>` | 対象プロジェクトの選択 |
| `env [<名前>] [--create]` | 環境の一覧・作成・選択 |
| `var set/set-secret/list/kind/remove` | 変数の管理 (`kind` は PUBLIC → SECRET の変更) |
| `run <コマンド>` | PUBLIC 変数だけを渡してコマンドを実行 (Secret は渡りません) |
| `import [ファイル] [--yes] [--dry-run] [--delete]` | `.env` の取り込み |
| `clean [ディレクトリ] [--dry-run] [--yes]` | 取り込み済み `.env` の削除 |
| `export-example` / `export-env [ファイル]` | テンプレート / 実値ファイルの書き出し |
| `cred add/list/show/set/copy/remove` | 資格情報の管理 |
| `ssh <名前> [-- 引数]` | 保存した鍵で SSH |
| `connection add/list/remove` | 外部サービス接続 |
| `ai check/permit/rules/unpermit/approvals/approve/deny/clients/register` | AI の権限と承認 |
| `activity [--limit N]` | 監査ログ |
| `mcp --client <名前>` | MCP サーバーとして起動 |
| `scan [ディレクトリ]` | 漏えい検査 |
| `agent [--ping]` | Local Agent (Unix ソケット) |
| `vault status / key-backend <file\|keychain>` | マスターキーの保存先 |
| `config show / language / theme` | 設定 (Desktop と共通) |
| `status` | 現在の選択とデータの場所 |

共通オプション: `--json` (機械可読)、`--no-animation`、`-v` (詳細ログ)。

---

## 仕組みと安全性

### データの置き場所

すべて 1 つのディレクトリに入ります (`ENVENB_HOME` で変更可)。
**絶対に git にコミットしないでください。**

EnvEnb は macOS 専用です。

| OS | 場所 |
|---|---|
| macOS | `~/Library/Application Support/envenb/` |

| ファイル | 内容 |
|---|---|
| `envenb.db` | SQLite。PUBLIC は平文、SECRET は暗号文と nonce のみ |
| `master.key` | 32 byte のマスターキー (`0600`)。DB と別に置くので、DB のコピーだけでは復号できません |
| `state.json` | 選択中のプロジェクト / 環境の id |

### なぜ実値の `.env` を残さないのか

Claude Code などの AI は作業ディレクトリのファイルを読みます。平文の `.env.local` が
隣にあれば、Vault と Broker で分離した意味がありません。値は EnvEnb に置き、
アプリには `envenb run` で渡すのが基本形です。

### Secret の扱い

- **SQLite に平文で入りません。** Rust 側で XChaCha20-Poly1305 により暗号化してから保存し、
  `secrets` テーブルには平文カラム自体がありません。
- **値を返す API がありません。** CLI・Desktop・MCP のいずれにも `get_secret` 相当はなく、
  復号は Rust クレート内部に閉じています。
- **平文が渡るのは登録時の 1 回だけ。** 受け取った直後に Secret 専用型へ移し、元のバッファは消去します。
- **ログに値は出ません。** エラーメッセージも変数名やパスだけを含みます。

### AI との境界

- 平文を出すコマンド (`run` / `export-env` / `ssh` / `cred copy` / `var copy`) は **人間専用** です。
  対話端末からのみ実行でき、`CLAUDECODE` などエージェントのセッション内では拒否されます。
  自分で書いたスクリプトからは `ENVENB_ALLOW_UNATTENDED=1` で許可できます。
- `var list` や `status` のようなメタデータだけのコマンドは AI からも使えます。
- 「管理者が許可した」と AI が主張しても、判定に使うのは EnvEnb のルールだけです。

### 運用の推奨

- API キーやトークンは必ず SECRET に。`NEXT_PUBLIC_*` のようにブラウザへ出る値だけ PUBLIC に。
- マスターキーは Keychain に置く (`envenb vault key-backend keychain`)。macOS の新規 Vault は既定でこちらです。
- Vault を別マシンへ移すときは `envenb.db` と `master.key` を **別経路で** 運ぶ。

### まだできないこと

| 機能 | 状態 |
|---|---|
| AWS SSO / AssumeRole | 未実装 (静的アクセスキーの SigV4 は対応) |
| Secret の手動 Reveal (Touch ID 付き) | 未実装 |
| Windows / Linux 対応 | 対象外 (macOS 専用) |
| Playwright による E2E テスト | 未実装 (Vitest の単体テストはあり) |

---

## 困ったとき

| 症状 | 対処 |
|---|---|
| `envenb: command not found` | `~/.cargo/bin` が PATH にありません ([インストール](#1-インストール)) |
| `no project selected` / `no environment selected` | エラーに次のコマンドと候補が出ます。`envenb use <名前>` → `envenb env <名前>` |
| `no projects yet` | `envenb project add <名前> --path .` |
| `a project with that name already exists` | 名前は大文字小文字を区別しません。`envenb project list` で確認 |
| `secret could not be decrypted` | `master.key` が DB と対になっていません。両方を一緒に扱ってください |
| 同上 + `vault status` が keychain | OS が Keychain アクセスを拒否した可能性。ダイアログで「常に許可」か `key-backend file` に戻す |
| `permission denied` / `approval timed out` (AI 側) | `envenb ai check` で判定を確認。ASK なら `envenb ai approvals` → `approve` |
| `connection has no credential configured` | `envenb connection add --secret <SECRET 名>` で認証情報を紐づける |
| `refused inside an AI agent session` | 仕様です。平文を出すコマンドは人間のターミナルからのみ実行できます |
| Desktop が白い画面 | `pnpm dev` で起動しているか確認 |
| 舞妓が出ない | TTY か、`CI` / `ENVENB_NO_ANIMATION` / `--json` が無いか確認 |
| 詳しいログを見たい | `envenb -v ...` または `RUST_LOG=envenb=debug` |
