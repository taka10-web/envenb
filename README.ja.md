# EnvEnb (日本語)

EnvEnb は、案件ごとの環境変数・Secret・(将来的に) クラウド接続を **完全ローカル**
で管理し、Claude Code / Codex などの AI エージェントには Secret 本体を見せず、
Broker 経由で許可された操作だけを実行させるための開発者ツールです。

**実装済み (MVP v0.1)**

- Project / Environment / Variable (PUBLIC・SECRET) の管理
- Vault: XChaCha20-Poly1305 で暗号化し SQLite に保存。`secrets` テーブルには平文カラム自体がない
- Master Key と Vault の責務分離 (`MasterKeyProvider` trait。Phase 1 は `0600` のファイル、後続で OS Keychain)
- CLI `envenb`: `project list/add/remove`、`use`、`env`、`var list/set/set-secret/remove`、`status`。`--json`、`--no-animation`。ヘルプとメッセージは日本語対応 (`ENVENB_LANG=ja` または `LANG`)。引数なしで usage を表示
- 使い方の手順書: [USAGE.md](USAGE.md)
- Desktop (Tauri 2 + React): Projects / Project Detail / Environments / Variables。Connections / AI Access / Activity はルートとナビだけ先行配置。**日本語・英語切り替え対応**
- `envenb run`: 復号した Secret を子プロセスの環境変数にだけ注入
- `.env` Import (PUBLIC / SECRET を自動分類し、TTY では 1 件ずつ確認) と `.env.example` Export
- Connection (generic_http / openai / supabase)。認証情報は SECRET 変数の「名前」で参照
- AI Permission Engine: クライアント × プロジェクト × 環境 × 接続 × 操作 (READ / WRITE / DELETE) で ALLOW / ASK / DENY。既定は development が READ 許可・WRITE 確認・DELETE 拒否、production が READ 確認・他は拒否
- ASK の承認フロー: Desktop の AI アクセス画面または `envenb ai approve` で人間が判断 (3 分でタイムアウト)
- Secret Broker: 認証ヘッダーを注入して外部 API を呼び、レスポンス中の認証情報を `[REDACTED]` に置換
- MCP Server (`envenb mcp`): Claude Code / Codex 向け。ツールは一覧系と `call_service` / `supabase_select` のみで、Secret を返すツールは存在しない
- 監査ログ (`envenb activity` / Desktop の Activity)
- マスターキーの OS Keychain 保存 (`envenb vault key-backend keychain`)
- 設定 (言語 ja / en / system、テーマ light / dark / system) を CLI と Desktop で共有
- ドット絵の舞妓 (赤・錦・黒) を CLI の節目コマンドで表示。非 TTY / CI / `--json` / `--no-animation` では出さず、`NO_COLOR` 対応
- Connector 追加: Cloudflare / Vercel / GitHub (Bearer)、AWS (SigV4 署名を Broker 内で実施。AWS 公式のテストベクタで検証)
- Credential (資格情報): テストアカウント (URL・ユーザー名・パスワード・TOTP)、SSH (ホスト・ユーザー・秘密鍵)、データベース、ファイル/証明書をフィールド単位で暗号化保存。`envenb cred copy` / Desktop のコピーボタンでクリップボードへ (30 秒で自動消去、値は画面に出さない)、`envenb ssh`、`envenb run --with-credentials`。AI には `list_credentials` で名前と非秘密フィールドのみ
- `envenb scan`: 作業ツリー内に Secret の値が漏れていないか、追跡中の `.env` が無いかを検査 (値は表示しない)
- `envenb agent`: Local Agent を Unix ソケット (0600) で起動
- GitHub Actions: CI (fmt / clippy / test / typecheck / vitest / build) とタグ時のリリースビルド
- Desktop でも同じドット舞妓を使用。サイドバーのロゴは舞妓の顔、空状態には三人の舞妓が並び、読み込み中や保存中は舞うローディング表示 (`MaikoLoader` / `MaikoInline`)。アプリアイコンも同じ顔のスプライト

**セキュリティ上の判断**

- Secret 型 (`SecretValue`, `MasterKey`) は `Debug` で `[REDACTED]`、`Display`/`Serialize`/`Clone` なし、drop 時に zeroize
- AI から到達しうる「Secret を返す API」を作らない。`EnvEnb` の復号メソッドは `pub(crate)`、Agent の要求型にも該当バリアントなし
- 平文が webview から Rust に渡るのは `set_secret_variable` の 1 回のみ。受け取り直後に `SecretValue` へ移し、元バッファを zeroize
- CLI は Secret を argv から受け取らない (stdin、TTY ではエコー無効)
- ログ・エラーには識別子のみ。値は載らない

**Claude Code から使う**

```bash
claude mcp add envenb -- envenb mcp --client claude-code
```

**未実装**: Touch ID / Windows Hello 付きの手動 Reveal (OS 認証を組み込むまでは Reveal 自体を置かない方針)、AWS SSO / AssumeRole、Windows の名前付きパイプ、Playwright E2E。詳細は上記英語セクション参照。

---

詳しい手順は [USAGE.md](USAGE.md) にあります。
