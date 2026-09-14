use clap::{ArgAction, Args, Command as ClapCommand, Parser, Subcommand};

use crate::i18n::{Lang, lang, tr};

fn help_template() -> &'static str {
    match lang() {
        Lang::En => "{about}\n\n\x1b[1mUsage:\x1b[0m {usage}\n\n{all-args}{after-help}",
        Lang::Ja => "{about}\n\n\x1b[1m使い方:\x1b[0m {usage}\n\n{all-args}{after-help}",
    }
}

/// EnvFish — local secrets, AI never sees them.
#[derive(Parser, Debug)]
#[command(
    name = "envfish",
    version,
    about = tr(
        "EnvFish — manage per-project variables and secrets locally. Your AI uses them; it never sees them.",
        "EnvFish — 案件ごとの環境変数と Secret をローカルで管理します。AI は使えても、中身は見られません。",
    ),
    help_template = help_template(),
    disable_help_subcommand = true,
    disable_help_flag = true,
    disable_version_flag = true,
    after_help = tr(
        "Quick start:\n  envfish project add <name> --path <dir>\n  envfish use <name>\n  envfish env development --create\n  envfish var set APP_URL http://localhost:3000\n  printf '%s' \"$KEY\" | envfish var set-secret OPENAI_API_KEY\n\nLanguage: ENVFISH_LANG=ja|en (defaults to LANG).",
        "クイックスタート:\n  envfish project add <名前> --path <ディレクトリ>\n  envfish use <名前>\n  envfish env development --create\n  envfish var set APP_URL http://localhost:3000\n  printf '%s' \"$KEY\" | envfish var set-secret OPENAI_API_KEY\n\n表示言語: ENVFISH_LANG=ja|en (既定は LANG に従います)。",
    ),
)]
pub struct Cli {
    #[arg(long, global = true, help = tr(
        "Disable the goldfish animation (also: ENVFISH_NO_ANIMATION=1, CI=1, non-TTY)",
        "金魚アニメーションを無効化 (ENVFISH_NO_ANIMATION=1、CI=1、非 TTY でも無効)",
    ))]
    pub no_animation: bool,

    #[arg(long, global = true, help = tr(
        "Emit machine-readable JSON instead of tables",
        "表形式ではなく JSON で出力",
    ))]
    pub json: bool,

    #[arg(short, long, global = true, help = tr("Verbose logging to stderr", "詳細ログを stderr に出力"))]
    pub verbose: bool,

    #[arg(short, long, global = true, action = ArgAction::Help, help = tr("Print help", "ヘルプを表示"))]
    pub help: Option<bool>,

    #[arg(short = 'V', long, action = ArgAction::Version, help = tr("Print version", "バージョンを表示"))]
    pub version: Option<bool>,

    /// Omitted → show the splash and usage.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Apply the locale-dependent template and headings to a command and all of its
/// subcommands. clap does not propagate these, so we walk the tree once at startup.
pub fn localize(cmd: ClapCommand) -> ClapCommand {
    let options_heading = tr("Options", "オプション");
    cmd.help_template(help_template())
        .subcommand_help_heading(tr("Commands", "コマンド"))
        .mut_args(move |arg| {
            if arg.is_positional() {
                arg
            } else {
                arg.help_heading(options_heading)
            }
        })
        .mut_subcommands(localize)
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(about = tr("Manage projects", "プロジェクトを管理"))]
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    #[command(about = tr(
        "Show data directory, vault and current selection",
        "データディレクトリ・Vault・現在の選択を表示",
    ))]
    Status,
    #[command(about = tr("Select the current project (by name or id)", "現在のプロジェクトを選択 (名前または id)"))]
    Use {
        #[arg(value_name = "PROJECT", help = tr("Project name or id", "プロジェクト名または id"))]
        project: String,
    },
    #[command(about = tr(
        "List environments of the current project, or select one",
        "現在のプロジェクトの環境を一覧、または選択",
    ))]
    Env(EnvArgs),
    #[command(about = tr(
        "Manage variables of the current project/environment",
        "現在のプロジェクト/環境の変数を管理",
    ))]
    Var {
        #[command(subcommand)]
        command: VarCommand,
    },
    #[command(about = tr(
        "Run a command with the current environment's variables and secrets injected",
        "現在の環境の変数と Secret を注入してコマンドを実行",
    ), trailing_var_arg = true)]
    Run {
        #[arg(long, help = tr(
            "Also expose credentials: ENVFISH_CRED_<NAME>_<FIELD> for account/database/ssh fields and ENVFISH_FILE_<NAME> paths for file credentials (0600 temp files, removed on exit)",
            "資格情報も渡す: account/database/ssh は ENVFISH_CRED_<名前>_<フィールド>、file は ENVFISH_FILE_<名前> にパス (0600 の一時ファイル、終了時に削除)",
        ))]
        with_credentials: bool,
        #[arg(value_name = "COMMAND", required = true, num_args = 1.., help = tr(
            "Command and arguments, e.g. `envfish run pnpm dev`",
            "コマンドと引数。例: `envfish run pnpm dev`",
        ))]
        command: Vec<String>,
    },
    #[command(about = tr(
        "Import a .env file into the current environment (PUBLIC/SECRET classified, confirmed before saving)",
        ".env を現在の環境に取り込み (PUBLIC/SECRET を分類し、保存前に確認)",
    ))]
    Import {
        #[arg(value_name = "FILE", default_value = ".env")]
        file: String,
        #[arg(long, help = tr("Accept the suggested classification without asking", "分類の提案をそのまま受け入れる"))]
        yes: bool,
        #[arg(long, help = tr("Show the plan only; store nothing", "計画のみ表示し、保存しない"))]
        dry_run: bool,
        #[arg(long, help = tr("Do not add the file to .gitignore", ".gitignore に追記しない"))]
        no_gitignore: bool,
        #[arg(long, help = tr(
            "Delete the file after a successful import (only when every variable in it is now stored)",
            "取り込み成功後にファイルを削除 (ファイル内の全変数が保存済みの場合のみ)",
        ))]
        delete: bool,
    },
    #[command(about = tr(
        "Delete .env files in the project directory whose variables are all stored in EnvFish",
        "全変数が EnvFish に保存済みの .env ファイルをプロジェクトディレクトリから削除",
    ))]
    Clean {
        #[arg(value_name = "DIR", help = tr("Directory to scan (default: the project's local path, else .)", "対象ディレクトリ (既定: プロジェクトのローカルパス、無ければ .)"))]
        dir: Option<String>,
        #[arg(long, help = tr("Show what would be deleted without deleting", "削除せず対象だけ表示"))]
        dry_run: bool,
        #[arg(long, help = tr("Delete without asking for each file", "ファイルごとの確認を省略"))]
        yes: bool,
    },
    #[command(about = tr(
        "Write a real .env file (values included) for tools that cannot use `envfish run`. 0600, refused if git tracks the path",
        "実値入りの .env を書き出す (`envfish run` が使えないツール向け)。0600 で作成し、git 追跡中のパスには書かない",
    ))]
    ExportEnv {
        #[arg(value_name = "FILE", default_value = ".env.local")]
        file: String,
        #[arg(long, help = tr("Overwrite an existing file", "既存ファイルを上書き"))]
        force: bool,
        #[arg(long, help = tr("Do not add the file to .gitignore", ".gitignore に追記しない"))]
        no_gitignore: bool,
    },
    #[command(about = tr(
        "Write a .env.example for the current environment (secrets blank)",
        "現在の環境の .env.example を出力 (Secret は空欄)",
    ))]
    ExportExample {
        #[arg(value_name = "FILE", default_value = ".env.example")]
        file: String,
    },
    #[command(about = tr("Manage external service connections", "外部サービス接続を管理"))]
    Connection {
        #[command(subcommand)]
        command: ConnectionCommand,
    },
    #[command(about = tr("AI clients, permissions and approvals", "AI クライアント・権限・承認"))]
    Ai {
        #[command(subcommand)]
        command: AiCommand,
    },
    #[command(about = tr("Show the audit log", "監査ログを表示"))]
    Activity {
        #[arg(long, default_value_t = 50, value_name = "N")]
        limit: i64,
    },
    #[command(about = tr(
        "Start the MCP server on stdio (for Claude Code / Codex)",
        "MCP サーバーを stdio で起動 (Claude Code / Codex 用)",
    ))]
    Mcp {
        #[arg(long, value_name = "NAME", default_value = "mcp-client", help = tr(
            "Name this AI client registers as (used for permissions and audit)",
            "この AI クライアントの登録名 (権限と監査に使用)",
        ))]
        client: String,
        #[arg(long, value_name = "KIND", default_value = "mcp")]
        kind: String,
    },
    #[command(about = tr(
        "Run the Local Agent on a Unix domain socket (<data dir>/agent.sock)",
        "Local Agent を Unix ドメインソケットで起動 (<データディレクトリ>/agent.sock)",
    ))]
    Agent {
        #[arg(long, help = tr("Send a ping to a running agent and exit", "起動中の Agent に ping を送って終了"))]
        ping: bool,
    },
    #[command(about = tr(
        "Scan a directory for leaked secret values and tracked .env files (values are never printed)",
        "ディレクトリ内の Secret 値の漏えいと追跡中の .env を検査 (値は表示しません)",
    ))]
    Scan {
        #[arg(value_name = "DIR", default_value = ".")]
        dir: String,
        #[arg(long, help = tr("Scan every environment of the current project, not just the selected one", "選択中の環境だけでなくプロジェクトの全環境を対象にする"))]
        all_environments: bool,
    },
    #[command(about = tr(
        "Credentials: test accounts, SSH targets, databases, files/certificates",
        "資格情報: テストアカウント、SSH、データベース、ファイル/証明書",
    ))]
    Cred {
        #[command(subcommand)]
        command: CredCommand,
    },
    #[command(about = tr(
        "Open an SSH session using a stored ssh credential (key written to a 0600 temp file, removed on exit)",
        "保存済み ssh 資格情報で SSH 接続 (鍵は 0600 の一時ファイルに展開し、終了時に削除)",
    ), trailing_var_arg = true)]
    Ssh {
        #[arg(value_name = "NAME")]
        name: String,
        #[arg(value_name = "ARGS", num_args = 0.., help = tr("Extra arguments passed to ssh", "ssh に渡す追加引数"))]
        args: Vec<String>,
    },
    #[command(about = tr("Settings shared with the desktop app", "Desktop と共有する設定"))]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    #[command(about = tr("Master key storage", "マスターキーの保存"))]
    Vault {
        #[command(subcommand)]
        command: VaultCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProjectCommand {
    #[command(about = tr("List projects", "プロジェクトを一覧"))]
    List,
    #[command(about = tr("Register a project", "プロジェクトを登録"))]
    Add {
        #[arg(value_name = "NAME", help = tr("Project name (unique, case-insensitive)", "プロジェクト名 (一意、大文字小文字は区別しない)"))]
        name: String,
        #[arg(long, value_name = "DIR", help = tr("Local checkout path", "ローカルのチェックアウトパス"))]
        path: Option<String>,
    },
    #[command(about = tr(
        "Delete a project and everything under it",
        "プロジェクトと配下のすべてを削除",
    ))]
    Remove {
        #[arg(value_name = "PROJECT", help = tr("Project name or id", "プロジェクト名または id"))]
        project: String,
    },
}

#[derive(Args, Debug)]
pub struct EnvArgs {
    #[arg(value_name = "ENVIRONMENT", help = tr(
        "Environment to select (by name or id). Omit to list environments",
        "選択する環境 (名前または id)。省略すると一覧表示",
    ))]
    pub environment: Option<String>,

    #[arg(long, help = tr(
        "Create the environment if it does not exist",
        "存在しない場合は環境を作成",
    ))]
    pub create: bool,
}

#[derive(Subcommand, Debug)]
pub enum VarCommand {
    #[command(about = tr(
        "List variables (secret values are never printed)",
        "変数を一覧 (Secret の値は表示されません)",
    ))]
    List,
    #[command(about = tr("Set a PUBLIC variable", "PUBLIC 変数を設定"))]
    Set {
        #[arg(value_name = "NAME", help = tr("Variable name ([A-Za-z_][A-Za-z0-9_]*)", "変数名 ([A-Za-z_][A-Za-z0-9_]*)"))]
        name: String,
        #[arg(value_name = "VALUE", help = tr("Plain value (visible to AI agents)", "平文の値 (AI エージェントから見えます)"))]
        value: String,
    },
    #[command(about = tr(
        "Set a SECRET variable. The value is read from stdin (never from argv, never echoed)",
        "SECRET 変数を設定。値は stdin から読み取ります (argv からは受け取らず、エコーもしません)",
    ))]
    SetSecret {
        #[arg(value_name = "NAME", help = tr("Variable name ([A-Za-z_][A-Za-z0-9_]*)", "変数名 ([A-Za-z_][A-Za-z0-9_]*)"))]
        name: String,
    },
    #[command(about = tr("Remove a variable", "変数を削除"))]
    Remove {
        #[arg(value_name = "NAME", help = tr("Variable name ([A-Za-z_][A-Za-z0-9_]*)", "変数名 ([A-Za-z_][A-Za-z0-9_]*)"))]
        name: String,
    },
}

// ---------------------------------------------------------------------------
// Phase 2 commands
// ---------------------------------------------------------------------------

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    #[command(about = tr("Show settings", "設定を表示"))]
    Show,
    #[command(about = tr("Set the display language: ja | en | system", "表示言語を設定: ja | en | system"))]
    Language {
        #[arg(value_name = "LANG")]
        language: String,
    },
    #[command(about = tr("Set the desktop theme: system | light | dark", "Desktop のテーマを設定: system | light | dark"))]
    Theme {
        #[arg(value_name = "THEME")]
        theme: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConnectionCommand {
    #[command(about = tr("List connections of the current environment", "現在の環境の接続を一覧"))]
    List,
    #[command(about = tr(
        "Add a connection. The credential is the NAME of a SECRET variable in this environment",
        "接続を追加。認証情報にはこの環境の SECRET 変数の名前を指定します",
    ))]
    Add {
        #[arg(value_name = "NAME", help = tr("Connection name (e.g. supabase)", "接続名 (例: supabase)"))]
        name: String,
        #[arg(long, value_name = "KIND", default_value = "generic_http", help = tr(
            "generic_http | openai | supabase | cloudflare | vercel | github | aws",
            "generic_http | openai | supabase | cloudflare | vercel | github | aws",
        ))]
        kind: String,
        #[arg(long = "meta", value_name = "KEY=VALUE", help = tr(
            "Non-secret metadata, repeatable. aws needs region=, service=, access_key_id_secret=<SECRET name>",
            "秘密でないメタデータ (複数指定可)。aws は region= service= access_key_id_secret=<SECRET 名> が必要",
        ))]
        meta: Vec<String>,
        #[arg(long, value_name = "URL", help = tr("Base URL (https)", "ベース URL (https)"))]
        url: Option<String>,
        #[arg(long, value_name = "SECRET_NAME", help = tr(
            "Name of the SECRET variable holding the credential",
            "認証情報を保持する SECRET 変数の名前",
        ))]
        secret: Option<String>,
        #[arg(long, value_name = "STYLE", help = tr(
            "bearer | header:<Name> | query:<name> | supabase | none",
            "bearer | header:<Name> | query:<name> | supabase | none",
        ))]
        auth: Option<String>,
    },
    #[command(about = tr("Remove a connection", "接続を削除"))]
    Remove {
        #[arg(value_name = "CONNECTION")]
        connection: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AiCommand {
    #[command(about = tr("List registered AI clients", "登録済み AI クライアントを一覧"))]
    Clients,
    #[command(about = tr("Register an AI client", "AI クライアントを登録"))]
    Register {
        #[arg(value_name = "NAME")]
        name: String,
        #[arg(long, value_name = "KIND", default_value = "mcp", help = tr(
            "claude_code | codex | mcp | other",
            "claude_code | codex | mcp | other",
        ))]
        kind: String,
    },
    #[command(about = tr(
        "Set a permission rule for the current project/environment",
        "現在のプロジェクト/環境に権限ルールを設定",
    ))]
    Permit {
        #[arg(value_name = "ACTION", help = tr("READ | WRITE | DELETE", "READ | WRITE | DELETE"))]
        action: String,
        #[arg(value_name = "DECISION", help = tr("ALLOW | ASK | DENY", "ALLOW | ASK | DENY"))]
        decision: String,
        #[arg(long, value_name = "CLIENT", help = tr("Client name or id (default: any)", "クライアント名または id (既定: すべて)"))]
        client: Option<String>,
        #[arg(long, value_name = "CONNECTION", help = tr("Connection name or id (default: any)", "接続名または id (既定: すべて)"))]
        connection: Option<String>,
        #[arg(long, help = tr("Apply to all environments of the project", "プロジェクトの全環境に適用"))]
        all_environments: bool,
    },
    #[command(about = tr("List permission rules", "権限ルールを一覧"))]
    Rules,
    #[command(about = tr("Delete a permission rule by id", "権限ルールを id で削除"))]
    Unpermit {
        #[arg(value_name = "RULE_ID")]
        rule_id: String,
    },
    #[command(about = tr("Show effective decisions for the current scope", "現在のスコープの実効判定を表示"))]
    Check {
        #[arg(long, value_name = "CLIENT")]
        client: Option<String>,
        #[arg(long, value_name = "CONNECTION")]
        connection: Option<String>,
    },
    #[command(about = tr("List pending approvals", "保留中の承認要求を一覧"))]
    Approvals {
        #[arg(long, help = tr("Include resolved approvals", "解決済みも含める"))]
        all: bool,
    },
    #[command(about = tr("Approve a pending request (id or prefix)", "保留中の要求を承認 (id または先頭部分)"))]
    Approve {
        #[arg(value_name = "APPROVAL_ID")]
        id: String,
    },
    #[command(about = tr("Deny a pending request (id or prefix)", "保留中の要求を拒否 (id または先頭部分)"))]
    Deny {
        #[arg(value_name = "APPROVAL_ID")]
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum VaultCommand {
    #[command(about = tr("Show where the master key is stored", "マスターキーの保存場所を表示"))]
    Status,
    #[command(about = tr(
        "Move the master key: file | keychain (OS credential store)",
        "マスターキーの保存先を変更: file | keychain (OS の資格情報ストア)",
    ))]
    KeyBackend {
        #[arg(value_name = "BACKEND")]
        backend: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum CredCommand {
    #[command(about = tr("List credentials of the current environment", "現在の環境の資格情報を一覧"))]
    List,
    #[command(about = tr(
        "Add a credential. Secret fields are prompted for (hidden) or read with --field NAME=@file / NAME=-",
        "資格情報を追加。Secret フィールドは非表示入力、または --field 名=@ファイル / 名=- (stdin) で指定",
    ))]
    Add {
        #[arg(value_name = "NAME")]
        name: String,
        #[arg(long, value_name = "KIND", help = tr("account | ssh | database | file", "account | ssh | database | file"))]
        kind: String,
        #[arg(long = "field", value_name = "FIELD=VALUE", help = tr(
            "Field value. VALUE may be @path (file contents) or - (stdin). Repeatable",
            "フィールド値。VALUE は @パス (ファイル内容) または - (stdin) も可。複数指定可",
        ))]
        fields: Vec<String>,
        #[arg(long, value_name = "TEXT", help = tr("Non-secret memo", "メモ (秘密でないもの)"))]
        note: Option<String>,
    },
    #[command(about = tr("Show a credential (non-secret fields; secret fields as names)", "資格情報を表示 (Secret は名前のみ)"))]
    Show {
        #[arg(value_name = "NAME")]
        name: String,
    },
    #[command(about = tr("Set fields on an existing credential", "既存の資格情報のフィールドを設定"))]
    Set {
        #[arg(value_name = "NAME")]
        name: String,
        #[arg(long = "field", value_name = "FIELD=VALUE", required = true)]
        fields: Vec<String>,
    },
    #[command(about = tr(
        "Copy one secret field to the clipboard (TTY only, cleared after 30 s). Use --field totp for a one-time code",
        "Secret フィールドをクリップボードにコピー (TTY のみ、30 秒後に消去)。--field totp でワンタイムコード",
    ))]
    Copy {
        #[arg(value_name = "NAME")]
        name: String,
        #[arg(long, value_name = "FIELD", default_value = "password")]
        field: String,
    },
    #[command(about = tr("Remove a credential", "資格情報を削除"))]
    Remove {
        #[arg(value_name = "NAME")]
        name: String,
    },
}
