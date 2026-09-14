# EnvFish 使い方ガイド

EnvFish は、案件ごとの環境変数と Secret を **完全ローカル** で管理し、AI エージェント
(Claude Code / Codex など) には Secret の中身を見せずに使わせるためのツールです。
このガイドは現在の実装 (MVP v0.1) で実際にできることだけを書いています。

> AI に Secret を渡すのではなく、Secret を使う機能だけを渡す。

---

## 1. セットアップ

### 必要なもの

| ツール | バージョン | 備考 |
|---|---|---|
| Rust | stable (1.85 以上) | `mise use -g rust@stable` または rustup |
| Node.js | 22 以上 | |
| pnpm | 11 以上 | |
| Tauri 2 の前提ツール | macOS: Xcode Command Line Tools | Desktop アプリのビルドに必要。<https://v2.tauri.app/start/prerequisites/> |

### ビルド

```bash
git clone <このリポジトリ> envfish
cd envfish
pnpm install
cargo build
cargo test
```

`cargo build` で `target/debug/envfish` (CLI) が作られます。
パスに入れておくと以降のコマンドをそのまま実行できます。

```bash
# 例: シンボリックリンクを置く
ln -s "$PWD/target/debug/envfish" ~/.local/bin/envfish
```

### データの保存場所

すべてのデータは 1 つのディレクトリに置かれます。

| OS | 場所 |
|---|---|
| macOS | `~/Library/Application Support/envfish/` |
| Linux | `~/.local/share/envfish/` |
| Windows | `%APPDATA%\envfish\` |

| ファイル | 内容 |
|---|---|
| `envfish.db` | SQLite。PUBLIC 変数は平文、SECRET は暗号文と nonce のみ |
| `master.key` | 32 byte のマスターキー (`0600`)。**DB と別に置くことでコピーだけでは復号できません** |
| `state.json` | CLI が選択中のプロジェクト/環境の id |

環境変数 `ENVFISH_HOME=/path` で場所を変えられます。案件や検証用に Vault を分けたい時に使います。
このディレクトリは **絶対に Git にコミットしないでください**。

---

## 2. CLI の基本フロー

引数なしで `envfish` を実行すると、金魚のスプラッシュと使い方が表示されます。

```bash
envfish
envfish --help
envfish var --help
```

### 2.1 プロジェクトを登録する

```bash
envfish project add my-app --path ~/works/my-app
envfish project add "Goldfish App"
envfish project list
```

最初に登録したプロジェクトは自動的に「現在のプロジェクト」になります (一覧で `*`)。

### 2.2 プロジェクトと環境を選ぶ

```bash
envfish use my-app                 # 名前 (大文字小文字を区別しない) か id
envfish env                            # 環境の一覧
envfish env development --create       # なければ作って選択
envfish env staging --create
envfish env production --create
envfish env development                # 切り替え
```

`use` でプロジェクトを切り替えると、環境の選択は解除されます。

### 2.3 変数を登録する

**PUBLIC** (AI に見えてよい値) は引数で渡します。

```bash
envfish var set APP_URL http://localhost:3000
envfish var set AWS_REGION ap-northeast-1
```

**SECRET** は **stdin から** 渡します。argv には載せません (シェル履歴や `ps` に残さないため)。

```bash
# 対話: 入力はエコーされません
envfish var set-secret OPENAI_API_KEY

# パイプ: 1Password / .env / 他ツールから渡す
op read "op://Dev/OpenAI/credential" | envfish var set-secret OPENAI_API_KEY
printf '%s' "$SUPABASE_SECRET_KEY" | envfish var set-secret SUPABASE_SECRET_KEY
```

変数名は `[A-Za-z_][A-Za-z0-9_]*` に限ります。同じ名前を PUBLIC と SECRET の両方で持つことはできず、
種別を変えたい時はいったん `remove` してください。

### 2.4 確認する

```bash
envfish var list
```

```text
my-app / development
NAME            KIND    VALUE
--------------  ------  ---------------------
APP_URL         PUBLIC  http://localhost:3000
OPENAI_API_KEY  SECRET  ••••••••
```

SECRET の値を表示するコマンドは **存在しません**。これは仕様です (後述)。

```bash
envfish status          # データの場所、鍵の場所、件数、選択中のプロジェクト/環境
envfish var remove OPENAI_API_KEY
envfish project remove "Goldfish App"    # 配下の環境・変数・Secret も削除
```

### 2.5 スクリプトから使う

`--json` を付けると装飾なしの JSON になります。金魚も出ません。

```bash
envfish project list --json | jq '.[].name'
envfish var list --json | jq '.[] | select(.kind == "SECRET") | .name'
envfish status --json
```

`--json` でも SECRET の `value` は常に `null` です。

### 2.6 表示言語と演出

| 環境変数 / フラグ | 効果 |
|---|---|
| `envfish config language ja\|en\|system` | ヘルプとメッセージの言語 (Desktop と共通の設定)。`ENVFISH_LANG` 環境変数が最優先、次に設定、最後に `LANG` |
| `envfish config theme light\|dark\|system` | Desktop のテーマ (CLI からも変更可) |
| `--no-animation` / `ENVFISH_NO_ANIMATION=1` | 金魚アニメーションを出さない |
| `CI=1`、非 TTY (パイプ・リダイレクト)、`TERM=dumb`、`--json` | 自動的に出さない |
| `NO_COLOR=1` | 色を付けない (金魚は単色のドット) |

### 2.7 `.env` を取り込む / 書き出す

```bash
envfish import .env              # 1 件ずつ PUBLIC / SECRET / skip を確認 (Enter で提案どおり)
envfish import .env --yes        # 提案どおりに一括取り込み
envfish import .env --dry-run    # 計画だけ表示
envfish export-example           # .env.example を出力 (SECRET は空欄)
```

分類の目安: `NEXT_PUBLIC_*` `VITE_*` などは PUBLIC、`*_SECRET` `*_TOKEN` `*_API_KEY` `PASSWORD` や
`user:pass@host` を含む URL は SECRET、`*_KEY` は「要確認」として SECRET を提案します。
取り込み後は `.env` を `.gitignore` に入れ、削除を検討してください。

**`.env.local` はどうするか**

1. 取り込む: `envfish import .env.local` (development など対象の環境を選んでから)
2. `.gitignore` への追記は自動で行われます (`--no-gitignore` で抑止)
3. ファイルを消す: `envfish import .env.local --delete` (取り込み直後に確認して削除) または後から `envfish clean`。
   ファイル内のすべての変数が EnvFish に保存済みのときだけ削除され、`.env.example` などのテンプレートには触りません。
   Desktop では取り込み完了画面の「削除」ボタンから同じことができます (確認ダイアログ付き)
4. アプリは `envfish run pnpm dev` で起動する。Next.js / Vite などは `.env.local` より環境変数を優先するため挙動は変わりません

```bash
envfish clean --dry-run     # 削除対象と、未保存の変数が残っているファイルを表示
envfish clean               # 1 ファイルずつ y/N で確認
envfish clean --yes         # 確認なし (CI 等)
```

**なぜ実値の `.env` を置いたままにしないのか**: Claude Code などの AI は作業ディレクトリのファイルを読みます。
平文の `.env.local` が隣にあると、Vault と Broker で分離した意味がなくなります。値は EnvFish に置き、
アプリには `envfish run` で渡すのが基本形です。

ファイルしか読めないツール (エディタ拡張など) のために実値のファイルが必要なら、書き戻せます。

```bash
envfish export-env .env.local          # 0600 で作成し .gitignore に追記。git 追跡中のパスには書かない
envfish export-env .env.local --force  # 既存ファイルを上書き
```

### 2.8 Secret を注入してコマンドを実行する

```bash
envfish run pnpm dev
envfish run -- aws s3 ls        # オプションを含む場合は -- の後に
```

現在の環境の PUBLIC と復号した SECRET を **子プロセスの環境変数にだけ** 注入します。
`ENVFISH_PROJECT` / `ENVFISH_ENVIRONMENT` も渡します。値は画面にもログにも出ません。
AI エージェント自身を `envfish run` で起こすと環境変数から読めてしまうため、AI には次の
Broker 経由を使ってください。

### 2.9 外部サービスへの接続 (Connection)

認証情報は先に SECRET として登録し、接続には **その名前** を渡します。

```bash
printf '%s' "$SUPABASE_SERVICE_KEY" | envfish var set-secret SUPABASE_KEY
envfish connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY
envfish connection add openai   --kind openai   --secret OPENAI_API_KEY        # URL は既定値
envfish connection add myapi    --kind generic_http --url https://api.example.com --secret MYAPI_TOKEN --auth header:X-Api-Key
envfish connection list
```

`--auth` は `bearer` (既定) / `header:<ヘッダー名>` / `query:<パラメータ名>` / `supabase` / `none`。
ベース URL は https のみ (localhost だけ http 可) です。

種別と既定値:

| kind | 既定 URL | 認証 |
|---|---|---|
| `generic_http` | 必須 | bearer など |
| `openai` | https://api.openai.com/v1 | bearer |
| `supabase` | 必須 | apikey + Bearer |
| `cloudflare` | https://api.cloudflare.com/client/v4 | bearer |
| `vercel` | https://api.vercel.com | bearer |
| `github` | https://api.github.com | bearer |
| `aws` | 必須 (サービスのエンドポイント) | SigV4。`--meta region= service= access_key_id_secret=<SECRET 名>` が必要で、`--secret` にはシークレットアクセスキーの SECRET 名 |

```bash
printf '%s' "$AWS_ACCESS_KEY_ID"     | envfish var set-secret AWS_ACCESS_KEY_ID
printf '%s' "$AWS_SECRET_ACCESS_KEY" | envfish var set-secret AWS_SECRET_ACCESS_KEY
envfish connection add sqs --kind aws --url https://sqs.ap-northeast-1.amazonaws.com \
  --secret AWS_SECRET_ACCESS_KEY --meta region=ap-northeast-1 --meta service=sqs \
  --meta access_key_id_secret=AWS_ACCESS_KEY_ID
```

### 2.10 AI クライアントの権限

判定は クライアント × プロジェクト × 環境 × 接続 × 操作 (READ / WRITE / DELETE) ごとに
ALLOW / ASK / DENY。ルールが無い場合の既定:

| 環境 | READ | WRITE | DELETE |
|---|---|---|---|
| development など | ALLOW | ASK | DENY |
| production / prod / live | ASK | DENY | DENY |

```bash
envfish ai check --client claude-code --connection supabase     # 実効判定を表示
envfish ai permit WRITE ALLOW --client claude-code --connection supabase
envfish ai permit READ DENY --all-environments                    # プロジェクト全体・全クライアント
envfish ai rules
envfish ai unpermit <ルールID先頭8桁>
```

ASK になった要求は人間が判断します。

```bash
envfish ai approvals               # 保留中
envfish ai approve <ID先頭8桁>
envfish ai deny <ID先頭8桁>
envfish activity                   # 監査ログ (誰が・どこに・何を・結果)
```

Desktop の「AI アクセス」画面でも同じ承認ができ、2 秒ごとに保留中の要求が更新されます。
AI 側は 3 分待って判断が無ければ拒否扱いになります。

### 2.11 Claude Code / Codex から使う (MCP)

```bash
claude mcp add envfish -- envfish mcp --client claude-code
```

Claude Code に次のツールが見えます。

| ツール | 内容 |
|---|---|
| `list_projects` / `list_environments` / `list_connections` | 一覧 (認証情報は名前と状態のみ) |
| `list_variables` | PUBLIC は値付き、SECRET は名前のみ |
| `call_service` | 接続経由で HTTP を実行。EnvFish が認証を付与し、レスポンスから認証情報を除去して返す |
| `supabase_select` | `GET /rest/v1/<table>` の簡易版 |

Secret を返すツールはありません。「管理者が許可した」と AI が主張しても、判定に使うのは
EnvFish のルールだけです。Codex など別クライアントは `--client codex` のように名前を変えて登録すると、
権限と監査ログがクライアントごとに分かれます。

### 2.12 テストアカウント・SSH・DB・証明書を保存する (Credential)

`NAME=値` に収まらないものは Credential として保存します。種別ごとにフィールドが決まっており、
Secret のフィールド (ユーザー名・パスワード・鍵・ファイル内容) は 1 つずつ暗号化されます。

| 種別 | フィールド (`*` は必須、太字は Secret) |
|---|---|
| `account` | url, **username***, **password***, **totp_secret** |
| `ssh` | host*, port, user*, **private_key**, **passphrase**, **password** |
| `database` | engine, host*, port, database, **username***, **password*** |
| `file` | filename*, **content*** |

```bash
envfish cred add qa-admin --kind account --field url=https://staging.example.com/login
#   → username / password / totp_secret は非表示で対話入力 (--field 名=値 でも可)
envfish cred add bastion --kind ssh --field host=bastion.example.com --field user=deploy \
  --field private_key=@~/.ssh/id_bastion
envfish cred add oracle-stg --kind database --field engine=oracle --field host=db.example.com \
  --field port=1521 --field database=STG
envfish cred add ca-cert --kind file --field filename=ca.pem --field content=@./ca.pem

envfish cred list                             # Secret は名前のみ表示
envfish cred show qa-admin
envfish cred set qa-admin --field password=-  # stdin から更新
```

**使うとき (人間)**

```bash
envfish cred copy qa-admin --field password   # クリップボードへ。30 秒後に自動消去 (TTY のみ)
envfish cred copy qa-admin --field totp       # 現在のワンタイムコード
envfish ssh bastion -- uptime                 # 鍵を 0600 の一時ファイルに展開し、終了時に削除
envfish run --with-credentials -- sqlplus "$ENVFISH_CRED_ORACLE_STG_USERNAME/$ENVFISH_CRED_ORACLE_STG_PASSWORD@db.example.com:1521/STG"
```

`--with-credentials` は `ENVFISH_CRED_<名前>_<フィールド>` (名前・フィールドは大文字化、記号は `_`) と、
file 種別の `ENVFISH_FILE_<名前>` (一時ファイルのパス) を子プロセスにだけ渡します。

Desktop の「資格情報」画面でも同じ操作ができ、コピーボタンは Rust 側でクリップボードに書くため
値は画面にも webview にも出ません。AI に見えるのは `list_credentials` の名前と非秘密フィールドだけです。

### 2.13 漏えいを検査する

```bash
envfish scan                    # カレントディレクトリ。選択中の環境の Secret を対象
envfish scan ~/works/my-app --all-environments
```

保存済みの Secret の値がファイル内に現れていないか、`.env` が git に追跡されていないかを調べます。
値そのものは表示せず、`ファイル:行  変数名` だけを出します。見つかると終了コード 2 なので、
pre-commit フックや CI にも組み込めます。

### 2.14 Local Agent

```bash
envfish agent          # <データディレクトリ>/agent.sock で待ち受け (0600)
envfish agent --ping   # 起動確認
```

一覧・承認・監査・権限判定を JSON 行で提供します。Secret を返す要求は存在しません。

### 2.15 マスターキーを OS Keychain に移す

```bash
envfish vault status
envfish vault key-backend keychain    # ファイル → macOS Keychain / Windows 資格情報 / Linux Secret Service
envfish vault key-backend file        # 戻す
```

移行時は鍵を書き込んだ後に読み戻して確認し、その後に元を削除します。Secret の再暗号化は不要です。

---

## 3. Desktop アプリ

### 起動

```bash
pnpm dev             # Vite + Tauri を同時に起動 (ホットリロード)
pnpm desktop:build   # 配布用バンドル (apps/desktop/src-tauri/target/release/bundle/)
```

> debug ビルドの `envfish-desktop` バイナリは Vite 開発サーバー (port 1420) を読みます。
> 単体で起動すると白い画面になるので、必ず `pnpm dev` を使ってください。
> WebKit のインスペクタは `ENVFISH_DEVTOOLS=1 pnpm dev` で開きます。

CLI と Desktop は同じデータディレクトリを読むため、片方で登録した内容はもう片方にそのまま出ます。

### 画面

| 画面 | できること |
|---|---|
| プロジェクト | 一覧、登録 (名前とローカルパス)、カードから詳細へ |
| プロジェクト詳細 › 環境 | 環境の追加 (development / staging / production はクイック追加)、削除 |
| プロジェクト詳細 › 変数 | 環境を切り替えながら PUBLIC / SECRET の追加・削除。SECRET はパスワード欄で入力し、保存後は `••••••••` 表示のみ。`.env` のファイル選択または貼り付けによる取り込みと `.env.example` のコピー |
| 接続 | 環境ごとの接続一覧と追加 (種別・URL・認証情報の SECRET 名・方式) |
| 資格情報 | テストアカウント / SSH / DB / ファイルの登録と、フィールド単位のコピー (30 秒で消去)、TOTP コードのコピー |
| AI アクセス | 保留中の承認要求 (承認 / 拒否)、AI クライアントの登録、READ / WRITE / DELETE × ALLOW / ASK / DENY のマトリクス、明示ルールの一覧 |
| アクティビティ | 監査ログ (時刻・クライアント・PJ/環境・接続・操作・結果) |
| 使い方 | このガイドの要約。CLI コマンドはコピー可、既定の判定表付き |
| 設定 | 言語 (日本語 / English / システム)、テーマ (ライト / ダーク / システム)、Vault の情報 |

言語とテーマは設定画面で切り替えられ、CLI の `envfish config` と同じ値を共有します
(初回は OS の言語とテーマに従います)。読み込み中や保存中は 3 匹のドット金魚 (赤・錦・黒出目金) が泳ぎます。

---

## 4. Secret に関して知っておくこと

- **SECRET は SQLite に平文で入りません。** Rust 側で XChaCha20-Poly1305 により暗号化してから保存します。
  `secrets` テーブルには平文カラム自体がありません。
- **鍵は DB と別ファイル** (`master.key`) です。Phase 1 ではファイル保存のため、同じユーザーで動く
  他プロセスからは読めます。OS Keychain への移行は Phase 2 の最初の項目です。
- **値を返す API はありません。** CLI・Desktop・将来の MCP のいずれにも `get_secret` 相当はなく、
  復号は Rust クレート内部 (`pub(crate)`) に閉じています。
- **平文が渡るのは登録時の 1 回だけ。** CLI は stdin、Desktop はパスワード欄 → Rust。受け取った直後に
  Secret 専用型へ移し、元のバッファは消去します。
- **ログに値は出ません。** エラーメッセージも変数名やパスだけを含みます。

### 使い方の推奨

- API キーやトークンは必ず SECRET に。`NEXT_PUBLIC_*` のようにブラウザに出る値だけ PUBLIC に。
- `production` の Secret は `development` と同じ名前で別環境に登録しておくと、
  後続 Phase の `envfish run` や Broker で環境を切り替えるだけで済みます。
- Vault を別マシンへ移すときは `envfish.db` と `master.key` を **別経路で** 運んでください。

---

## 5. まだできないこと

| 機能 | 状態 |
|---|---|
| AWS SSO / AssumeRole | 未実装 (静的アクセスキーの SigV4 は対応) |
| Secret の手動 Reveal (Touch ID / Windows Hello 付き、人間のみ) と、それに伴うクリップボード自動消去 | 未実装 |
| Windows の名前付きパイプ (Local Agent) | 未実装 |
| Playwright による E2E テスト | 未実装 (Vitest の単体テストはあり) |

---

## 6. トラブルシューティング

| 症状 | 対処 |
|---|---|
| `no project selected` | `envfish use <プロジェクト>` を実行 |
| `no environment selected` | `envfish env <環境>` (なければ `--create`) |
| `a project with that name already exists` | 名前は大文字小文字を区別しません。`project list` で確認 |
| `secret could not be decrypted` | `master.key` が別物です。DB と鍵は対で扱ってください |
| Desktop が白い画面 | `pnpm dev` で起動しているか確認 (上記 §3) |
| `permission denied` / `approval timed out` (AI 側) | `envfish ai check` で判定を確認。ASK なら `envfish ai approvals` → `approve`。ルール変更は `ai permit` |
| `connection has no credential configured` | `connection add --secret <SECRET名>` で認証情報を紐づける |
| `secret could not be decrypted` の後に `vault status` が keychain | OS が Keychain アクセスを拒否した可能性。ダイアログで「常に許可」を選ぶか `key-backend file` に戻す |
| 金魚が出ない | TTY か、`CI` / `ENVFISH_NO_ANIMATION` / `--json` が無いか確認 |
| 詳しいログを見たい | `envfish -v ...` または `RUST_LOG=envfish=debug` |
