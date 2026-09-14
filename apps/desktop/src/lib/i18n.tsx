import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, queryKeys } from "./api";
import type { Settings } from "./types";

// Lightweight i18n: a flat dictionary per locale and a hook. Adding a language
// means adding one object below; adding a string means adding a key to both.
//
// The language *setting* ("ja" | "en" | "system") is shared with the CLI via
// settings; localStorage only caches it so the first paint is already right.

export type Locale = "en" | "ja";
export type LanguageSetting = Settings["language"];

const en = {
  "nav.variables": "Variables",
  "nav.credentials": "Credentials",
  "nav.connections": "Connections",
  "nav.aiAccess": "AI Access",
  "nav.activity": "Activity",
  "nav.projects": "Projects",
  "nav.settings": "Settings",
  "nav.guide": "Guide",
  "guide.contents": "Contents",
  "lang.label": "Language",

  "context.noProject": "No project",
  "context.noEnvironment": "No environment",
  "guide.needProject": "Create a project to get started.",
  "guide.needEnvironment": "This project has no environment yet.",

  "common.loading": "Loading…",
  "common.delete": "Delete",
  "common.cancel": "Cancel",
  "common.save": "Save",
  "common.add": "Add",
  "common.close": "Close",
  "common.name": "Name",
  "common.value": "Value",
  "common.kind": "Kind",
  "common.path": "Path",
  "common.created": "Created",
  "common.unexpectedError": "Unexpected error",
  "common.environment": "Environment",
  "common.project": "Project",
  "common.connection": "Connection",
  "common.client": "Client",
  "common.action": "Action",
  "common.decision": "Decision",
  "common.time": "Time",
  "common.summary": "Summary",
  "common.approve": "Approve",
  "common.deny": "Deny",

  "projects.title": "Projects",
  "projects.localPath": "Local path (optional)",
  "projects.create": "Create project",
  "projects.newTitle": "New project",
  "projects.newHint": "Name it after the client or product. The local path is optional.",
  "projects.empty": "No projects yet.",
  "projects.created": "Created {date}",
  "projects.noPath": "no local path",
  "projects.notFound": "Project not found.",
  "projects.settingsAria": "Settings of {name}",
  "projects.footer": "{projects} projects · {secrets} secrets encrypted at rest · master key: {key}",
  "projects.confirmDelete": 'Delete "{name}" and all of its environments, variables and secrets?',
  "projects.danger": "Delete project",

  "envs.title": "Environments",
  "envs.count": "{count} environments",
  "envs.placeholder": "environment name",
  "envs.add": "Add environment",
  "envs.quickAdd": "quick add:",
  "envs.empty": "No environments yet.",
  "envs.confirmDelete": 'Delete environment "{name}" and its variables?',
  "envs.deleteAria": "Delete {name}",

  "vars.title": "Variables",
  "vars.secretPlaceholder": "sealed on save, never shown again",
  "vars.secretNote": "Secrets are encrypted in Rust before they reach SQLite. This app cannot read them back; AI agents never can.",
  "vars.empty": "No variables in this environment.",
  "vars.encryptedAtRest": "encrypted at rest",
  "vars.makeSecret": "Make secret",
  "vars.makeSecretHint": "Click to encrypt this value and hide it from AI agents",
  "vars.confirmMakeSecret": "Encrypt {name} and hide its value? This cannot be undone from here: a SECRET can never be turned back into a PUBLIC variable.",
  "vars.secretIsPermanent": "A secret cannot be turned back into a public variable. Delete it and set the value again if it is not secret.",
  "vars.confirmDelete": "Delete {name}?",
  "vars.copyExample": "Copy .env.example",
  "vars.copied": "Copied to clipboard.",
  "vars.copyFailed": "Could not access the clipboard.",
  "vars.import.button": "Import .env",
  "vars.import.emptyHeading": "Start by importing your .env or add variables one by one",
  "vars.import.stepsAria": "Import steps",
  "vars.import.step.load": "Load",
  "vars.import.step.review": "Review",
  "vars.import.step.import": "Import",
  "vars.import.target": "Into environment {name}",
  "vars.import.dropTitle": "Drop your .env file here",
  "vars.import.chooseFile": "Choose file…",
  "vars.import.pasteInstead": "Paste instead",
  "vars.import.paste": "Paste .env content",
  "vars.import.continue": "Continue",
  "vars.import.helper": "Typical files: .env, .env.local, .env.development. Nothing is stored until step 3.",
  "vars.import.fileLoaded": "Loaded {name}",
  "vars.import.skip": "Skip",
  "vars.import.skipAria": "Skip {name}",
  "vars.import.kindAria": "Kind of {name}",
  "vars.import.showValue": "Show value of {name}",
  "vars.import.hideValue": "Hide value of {name}",
  "vars.import.note": "Note",
  "vars.import.review": "review",
  "vars.import.reviewHint": "The name does not make clear whether this is a secret. Kept as SECRET unless you change it.",
  "vars.import.overwrite": "will overwrite",
  "vars.import.summary": "{publicCount} public · {secretCount} secret · {skippedCount} skipped",
  "vars.import.invalidLines": "Lines skipped as invalid: {lines}",
  "vars.import.nothing": "No variables found in the loaded text.",
  "vars.import.back": "Back",
  "vars.import.confirm": "Import {count} variables",
  "vars.import.doneTitle": "Import complete",
  "vars.import.report": "Imported {publicAdded} public and {secretAdded} secret variables · {skipped} skipped",
  "vars.import.skippedNames": "Skipped:",
  "vars.import.reminderTitle": "Add .env to .gitignore and consider deleting the file",
  "vars.import.reminderBody": "EnvEnb now holds these values.",
  "vars.import.gitignoreAdded": "Added {pattern} to the project's .gitignore. Consider deleting the file — EnvEnb now holds these values.",
  "vars.import.gitignoreAlready": ".gitignore already ignores this file. Consider deleting it — EnvEnb now holds these values.",
  "vars.import.deleteFile": "Delete {name} from the project",
  "vars.import.confirmDeleteFile": "Delete {name} from the project folder? Only allowed when every variable in it is already stored in EnvEnb.",
  "vars.import.fileDeleted": "Deleted {name}. The values now live only in EnvEnb.",
  "vars.import.fileKept": "Kept the file: these variables are not stored yet: {names}",
  "vars.import.fileNotDotenv": "Not a .env file; nothing deleted.",
  "vars.import.another": "Import another",
  "vars.import.close": "Close",

  "settings.title": "Settings",
  "settings.appearance.title": "Appearance",
  "settings.language.title": "Language",
  "settings.language.system": "Follow system",
  "settings.theme.title": "Theme",
  "settings.theme.light": "Light",
  "settings.theme.dark": "Dark",
  "settings.theme.system": "System",
  "settings.vault.title": "Vault",
  "settings.vault.dataDir": "Data directory",
  "settings.vault.databasePath": "Database",
  "settings.vault.masterKey": "Master key",
  "settings.vault.keyBackend": "Key backend",
  "settings.vault.projects": "Projects",
  "settings.vault.secrets": "Secrets",
  "settings.vault.keychainNote": "To move the master key into the OS keychain, run:",

  "connections.title": "Connections",
  "connections.empty": "No connections in this environment yet.",
  "connections.add": "Add connection",
  "connections.baseUrl": "Base URL",
  "connections.optional": "optional",
  "connections.credential": "Credential",
  "connections.noCredential": "none",
  "connections.authStyle": "Auth style",
  "connections.authStyleAuto": "default: {style}",
  "connections.credentialHint": "The credential must be stored as a SECRET variable in this environment first. Only its name is referenced.",
  "connections.noSecretsHint": "This environment has no SECRET variables yet. Add one on the Variables page to use it as a credential.",
  "connections.confirmDelete": 'Delete connection "{name}"?',
  "connections.kind.generic_http": "Generic HTTP",
  "connections.kind.openai": "OpenAI",
  "connections.kind.supabase": "Supabase",
  "connections.kind.cloudflare": "Cloudflare",
  "connections.kind.vercel": "Vercel",
  "connections.kind.github": "GitHub",
  "connections.kind.aws": "AWS (SigV4)",
  "connections.aws.region": "Region",
  "connections.aws.service": "Service",
  "connections.aws.accessKeyId": "Access key ID (SECRET)",
  "connections.aws.secretAccessKey": "Secret access key (SECRET)",

  "creds.title": "Credentials",
  "creds.empty": "No credentials in this environment yet.",
  "creds.add": "Add credential",
  "creds.update": "Update",
  "creds.note": "Note",
  "creds.optional": "optional",
  "creds.required": "required",
  "creds.copyCode": "Copy code",
  "creds.copied": "Copied · clears in {seconds}s",
  "creds.copyAria": "Copy {field} of {name}",
  "creds.copyCodeAria": "Copy one-time code of {name}",
  "creds.updateAria": "Update {name}",
  "creds.editing": "Updating {name}",
  "creds.editHint": "Leave a secret field empty to keep the stored value. Only the fields you fill in are sent.",
  "creds.secretHint": "Secret fields are encrypted in Rust before they reach SQLite. They are never shown again; use Copy when you need one.",
  "creds.loadFromFile": "Load from file",
  "creds.fileLoaded": "Loaded {name}",
  "creds.confirmDelete": 'Delete credential "{name}"?',
  "creds.kind.account": "Account",
  "creds.kind.ssh": "SSH",
  "creds.kind.database": "Database",
  "creds.kind.file": "File",
  "creds.field.url": "URL",
  "creds.field.username": "Username",
  "creds.field.password": "Password",
  "creds.field.totp_secret": "TOTP secret",
  "creds.field.host": "Host",
  "creds.field.port": "Port",
  "creds.field.user": "User",
  "creds.field.private_key": "Private key",
  "creds.field.passphrase": "Passphrase",
  "creds.field.engine": "Engine",
  "creds.field.database": "Database",
  "creds.field.filename": "File name",
  "creds.field.content": "Content",

  "ai.title": "AI Access",
  "ai.approvals.title": "Pending approvals",
  "ai.approvals.requested": "requested {time}",
  "ai.approvals.expires": "expires {time}",
  "ai.clients.title": "Clients",
  "ai.clients.description": "A client is registered automatically the first time it connects with",
  "ai.clients.register": "Register",
  "ai.clients.empty": "No clients registered yet.",
  "ai.clients.lastSeen": "Last seen",
  "ai.clients.never": "never",
  "ai.clients.confirmDelete": 'Remove client "{name}" and its rules?',
  "ai.permissions.title": "Permissions",
  "ai.permissions.effective": "Effective decision per action for the selected scope, defaults included.",
  "ai.scope.anyClient": "any client",
  "ai.scope.anyProject": "any project",
  "ai.scope.anyEnvironment": "any environment",
  "ai.scope.anyConnection": "any connection",
  "ai.decision.allow": "Allow",
  "ai.decision.ask": "Ask",
  "ai.decision.deny": "Deny",
  "ai.legend.title": "Defaults when no rule matches",
  "ai.legend.development": "development-like environments: READ allow · WRITE ask · DELETE deny",
  "ai.legend.production": "production-like environments: READ ask · WRITE deny · DELETE deny",
  "ai.legend.note": "More specific rules (client, environment, connection) win over broader ones.",
  "ai.rules.title": "Explicit rules",
  "ai.rules.empty": "No explicit rules. Defaults apply everywhere.",
  "ai.rules.confirmDelete": "Delete this rule?",

  "activity.title": "Activity",
  "activity.empty": "Nothing recorded yet.",
};

const ja: Record<keyof typeof en, string> = {
  "nav.variables": "変数",
  "nav.credentials": "資格情報",
  "nav.connections": "接続",
  "nav.aiAccess": "AI アクセス",
  "nav.activity": "アクティビティ",
  "nav.projects": "プロジェクト",
  "nav.settings": "設定",
  "nav.guide": "使い方",
  "guide.contents": "目次",
  "lang.label": "言語",

  "context.noProject": "プロジェクトなし",
  "context.noEnvironment": "環境なし",
  "guide.needProject": "まずプロジェクトを作成してください。",
  "guide.needEnvironment": "このプロジェクトには環境がまだありません。",

  "common.loading": "読み込み中…",
  "common.delete": "削除",
  "common.cancel": "キャンセル",
  "common.save": "保存",
  "common.add": "追加",
  "common.close": "閉じる",
  "common.name": "名前",
  "common.value": "値",
  "common.kind": "種別",
  "common.path": "パス",
  "common.created": "作成日",
  "common.unexpectedError": "予期しないエラー",
  "common.environment": "環境",
  "common.project": "プロジェクト",
  "common.connection": "接続",
  "common.client": "クライアント",
  "common.action": "操作",
  "common.decision": "判定",
  "common.time": "時刻",
  "common.summary": "概要",
  "common.approve": "承認",
  "common.deny": "拒否",

  "projects.title": "プロジェクト",
  "projects.localPath": "ローカルパス (任意)",
  "projects.create": "プロジェクトを作成",
  "projects.newTitle": "新しいプロジェクト",
  "projects.newHint": "案件やプロダクトの名前を付けます。ローカルパスは任意です。",
  "projects.empty": "プロジェクトはまだありません。",
  "projects.created": "作成日 {date}",
  "projects.noPath": "ローカルパス未設定",
  "projects.notFound": "プロジェクトが見つかりません。",
  "projects.settingsAria": "{name} の設定",
  "projects.footer": "プロジェクト {projects} 件 · 暗号化済み Secret {secrets} 件 · マスターキー: {key}",
  "projects.confirmDelete": "「{name}」と、その環境・変数・Secret をすべて削除しますか。",
  "projects.danger": "プロジェクトを削除",

  "envs.title": "環境",
  "envs.count": "環境 {count} 件",
  "envs.placeholder": "環境名",
  "envs.add": "環境を追加",
  "envs.quickAdd": "クイック追加:",
  "envs.empty": "環境はまだありません。",
  "envs.confirmDelete": "環境「{name}」とその変数を削除しますか。",
  "envs.deleteAria": "{name} を削除",

  "vars.title": "変数",
  "vars.secretPlaceholder": "保存時に暗号化され、以後表示されません",
  "vars.secretNote": "Secret は SQLite に届く前に Rust 側で暗号化されます。このアプリからも読み戻せず、AI エージェントは決して閲覧できません。",
  "vars.empty": "この環境に変数はありません。",
  "vars.encryptedAtRest": "暗号化保存",
  "vars.makeSecret": "SECRET にする",
  "vars.makeSecretHint": "クリックすると値を暗号化し、AI から隠します",
  "vars.confirmMakeSecret": "{name} を暗号化して値を隠しますか。ここからは元に戻せません (SECRET を PUBLIC に戻すことはできません)。",
  "vars.secretIsPermanent": "SECRET は PUBLIC に戻せません。秘密でない場合は削除して値を入れ直してください。",
  "vars.confirmDelete": "{name} を削除しますか。",
  "vars.copyExample": ".env.example をコピー",
  "vars.copied": "クリップボードにコピーしました。",
  "vars.copyFailed": "クリップボードにアクセスできませんでした。",
  "vars.import.button": ".env を取り込む",
  "vars.import.emptyHeading": "まず .env を取り込むか、変数を 1 件ずつ追加してください",
  "vars.import.stepsAria": "取り込みの手順",
  "vars.import.step.load": "読み込み",
  "vars.import.step.review": "確認",
  "vars.import.step.import": "取り込み",
  "vars.import.target": "取り込み先の環境: {name}",
  "vars.import.dropTitle": ".env ファイルをここにドロップしてください",
  "vars.import.chooseFile": "ファイルを選択…",
  "vars.import.pasteInstead": "貼り付けで取り込む",
  "vars.import.paste": ".env の内容を貼り付け",
  "vars.import.continue": "次へ",
  "vars.import.helper": "対象となるのは .env、.env.local、.env.development などのファイルです。手順 3 で確定するまで何も保存されません。",
  "vars.import.fileLoaded": "{name} を読み込みました",
  "vars.import.skip": "除外",
  "vars.import.skipAria": "{name} を除外",
  "vars.import.kindAria": "{name} の種別",
  "vars.import.showValue": "{name} の値を表示",
  "vars.import.hideValue": "{name} の値を隠す",
  "vars.import.note": "備考",
  "vars.import.review": "review",
  "vars.import.reviewHint": "名前からは秘密情報か判断できません。変更しない場合は SECRET として保存されます。",
  "vars.import.overwrite": "上書きされます",
  "vars.import.summary": "PUBLIC {publicCount} 件 · SECRET {secretCount} 件 · 除外 {skippedCount} 件",
  "vars.import.invalidLines": "不正な行としてスキップ: {lines} 行目",
  "vars.import.nothing": "読み込んだテキストに変数が見つかりませんでした。",
  "vars.import.back": "戻る",
  "vars.import.confirm": "{count} 件の変数を取り込む",
  "vars.import.doneTitle": "取り込みが完了しました",
  "vars.import.report": "PUBLIC {publicAdded} 件、SECRET {secretAdded} 件を取り込みました · スキップ {skipped} 件",
  "vars.import.skippedNames": "スキップ:",
  "vars.import.reminderTitle": ".env を .gitignore に追加し、ファイルの削除もご検討ください",
  "vars.import.reminderBody": "これらの値は EnvEnb が保持しています。",
  "vars.import.gitignoreAdded": "プロジェクトの .gitignore に {pattern} を追記しました。ファイルの削除も検討してください。値は EnvEnb が保持しています。",
  "vars.import.gitignoreAlready": ".gitignore は既にこのファイルを除外しています。ファイルの削除も検討してください。値は EnvEnb が保持しています。",
  "vars.import.deleteFile": "{name} をプロジェクトから削除",
  "vars.import.confirmDeleteFile": "プロジェクトフォルダの {name} を削除しますか。ファイル内のすべての変数が EnvEnb に保存済みの場合だけ削除されます。",
  "vars.import.fileDeleted": "{name} を削除しました。値は EnvEnb だけが保持しています。",
  "vars.import.fileKept": "ファイルは残しました。未保存の変数があります: {names}",
  "vars.import.fileNotDotenv": ".env ファイルではないため削除しませんでした。",
  "vars.import.another": "別のファイルを取り込む",
  "vars.import.close": "閉じる",

  "settings.title": "設定",
  "settings.appearance.title": "表示",
  "settings.language.title": "言語",
  "settings.language.system": "システムに従う",
  "settings.theme.title": "テーマ",
  "settings.theme.light": "ライト",
  "settings.theme.dark": "ダーク",
  "settings.theme.system": "システム",
  "settings.vault.title": "Vault",
  "settings.vault.dataDir": "データディレクトリ",
  "settings.vault.databasePath": "データベース",
  "settings.vault.masterKey": "マスターキー",
  "settings.vault.keyBackend": "鍵の保管先",
  "settings.vault.projects": "プロジェクト数",
  "settings.vault.secrets": "Secret 数",
  "settings.vault.keychainNote": "マスターキーを OS のキーチェーンへ移す場合は、次のコマンドを実行してください:",

  "connections.title": "接続",
  "connections.empty": "この環境に接続はまだありません。",
  "connections.add": "接続を追加",
  "connections.baseUrl": "ベース URL",
  "connections.optional": "任意",
  "connections.credential": "認証情報",
  "connections.noCredential": "なし",
  "connections.authStyle": "認証方式",
  "connections.authStyleAuto": "既定: {style}",
  "connections.credentialHint": "認証情報は先にこの環境の SECRET 変数として保存しておく必要があります。参照されるのは名前だけです。",
  "connections.noSecretsHint": "この環境には SECRET 変数がまだありません。認証情報として使うには、変数ページで追加してください。",
  "connections.confirmDelete": "接続「{name}」を削除しますか。",
  "connections.kind.generic_http": "Generic HTTP",
  "connections.kind.openai": "OpenAI",
  "connections.kind.supabase": "Supabase",
  "connections.kind.cloudflare": "Cloudflare",
  "connections.kind.vercel": "Vercel",
  "connections.kind.github": "GitHub",
  "connections.kind.aws": "AWS (SigV4)",
  "connections.aws.region": "リージョン",
  "connections.aws.service": "サービス",
  "connections.aws.accessKeyId": "アクセスキー ID (SECRET)",
  "connections.aws.secretAccessKey": "シークレットアクセスキー (SECRET)",

  "creds.title": "資格情報",
  "creds.empty": "この環境に資格情報はまだありません。",
  "creds.add": "資格情報を追加",
  "creds.update": "更新",
  "creds.note": "メモ",
  "creds.optional": "任意",
  "creds.required": "必須",
  "creds.copyCode": "コードをコピー",
  "creds.copied": "コピーしました · {seconds} 秒後に消去",
  "creds.copyAria": "{name} の {field} をコピー",
  "creds.copyCodeAria": "{name} のワンタイムコードをコピー",
  "creds.updateAria": "{name} を更新",
  "creds.editing": "{name} を更新しています",
  "creds.editHint": "Secret 項目を空のままにすると、保存済みの値が保持されます。入力した項目だけが送信されます。",
  "creds.secretHint": "Secret 項目は SQLite に届く前に Rust 側で暗号化されます。以後は表示されないため、必要なときはコピーを使ってください。",
  "creds.loadFromFile": "ファイルから読み込む",
  "creds.fileLoaded": "{name} を読み込みました",
  "creds.confirmDelete": "資格情報「{name}」を削除しますか。",
  "creds.kind.account": "アカウント",
  "creds.kind.ssh": "SSH",
  "creds.kind.database": "データベース",
  "creds.kind.file": "ファイル",
  "creds.field.url": "URL",
  "creds.field.username": "ユーザー名",
  "creds.field.password": "パスワード",
  "creds.field.totp_secret": "TOTP シークレット",
  "creds.field.host": "ホスト",
  "creds.field.port": "ポート",
  "creds.field.user": "ユーザー",
  "creds.field.private_key": "秘密鍵",
  "creds.field.passphrase": "パスフレーズ",
  "creds.field.engine": "エンジン",
  "creds.field.database": "データベース名",
  "creds.field.filename": "ファイル名",
  "creds.field.content": "内容",

  "ai.title": "AI アクセス",
  "ai.approvals.title": "承認待ち",
  "ai.approvals.requested": "要求 {time}",
  "ai.approvals.expires": "期限 {time}",
  "ai.clients.title": "クライアント",
  "ai.clients.description": "クライアントは、次のコマンドで初めて接続した時に自動登録されます:",
  "ai.clients.register": "登録",
  "ai.clients.empty": "登録済みのクライアントはまだありません。",
  "ai.clients.lastSeen": "最終接続",
  "ai.clients.never": "未接続",
  "ai.clients.confirmDelete": "クライアント「{name}」とそのルールを削除しますか。",
  "ai.permissions.title": "権限",
  "ai.permissions.effective": "選択したスコープで操作ごとに有効な判定です。既定値を含みます。",
  "ai.scope.anyClient": "すべてのクライアント",
  "ai.scope.anyProject": "すべてのプロジェクト",
  "ai.scope.anyEnvironment": "すべての環境",
  "ai.scope.anyConnection": "すべての接続",
  "ai.decision.allow": "許可",
  "ai.decision.ask": "確認",
  "ai.decision.deny": "拒否",
  "ai.legend.title": "ルールが一致しない場合の既定値",
  "ai.legend.development": "development 系の環境: READ 許可 · WRITE 確認 · DELETE 拒否",
  "ai.legend.production": "production 系の環境: READ 確認 · WRITE 拒否 · DELETE 拒否",
  "ai.legend.note": "より具体的なルール (クライアント・環境・接続) が、広いルールより優先されます。",
  "ai.rules.title": "明示的なルール",
  "ai.rules.empty": "明示的なルールはありません。すべて既定値が適用されます。",
  "ai.rules.confirmDelete": "このルールを削除しますか。",

  "activity.title": "アクティビティ",
  "activity.empty": "記録はまだありません。",
};

export type MessageKey = keyof typeof en;
const DICTS: Record<Locale, Record<MessageKey, string>> = { en, ja };

const STORAGE_KEY = "envenb.locale";

function systemLocale(): Locale {
  return navigator.language.toLowerCase().startsWith("ja") ? "ja" : "en";
}

function resolveLocale(setting: LanguageSetting): Locale {
  return setting === "system" ? systemLocale() : setting;
}

function readCached(): LanguageSetting {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored === "en" || stored === "ja" || stored === "system") return stored;
  } catch {
    /* storage unavailable */
  }
  return "system";
}

function writeCached(setting: LanguageSetting) {
  try {
    window.localStorage.setItem(STORAGE_KEY, setting);
  } catch {
    /* ignore */
  }
}

type Vars = Record<string, string | number>;

interface I18n {
  /** The stored preference ("system" follows navigator.language). */
  languageSetting: LanguageSetting;
  /** The locale actually used for rendering. */
  locale: Locale;
  setLocale: (l: LanguageSetting) => void;
  t: (key: MessageKey, vars?: Vars) => string;
}

const I18nContext = createContext<I18n | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const qc = useQueryClient();
  const [languageSetting, setLanguageSetting] = useState<LanguageSetting>(readCached);
  const settings = useQuery({ queryKey: queryKeys.settings, queryFn: api.getSettings });

  useEffect(() => {
    if (settings.data) {
      setLanguageSetting(settings.data.language);
      writeCached(settings.data.language);
    }
  }, [settings.data]);

  const locale = resolveLocale(languageSetting);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  const { mutate: persist } = useMutation({
    mutationFn: (l: LanguageSetting) => api.setLanguage(l),
    onSuccess: (data) => qc.setQueryData(queryKeys.settings, data),
  });

  const setLocale = useCallback(
    (l: LanguageSetting) => {
      setLanguageSetting(l);
      writeCached(l);
      persist(l);
    },
    [persist],
  );

  const t = useCallback(
    (key: MessageKey, vars?: Vars) => {
      let msg = DICTS[locale][key] ?? en[key] ?? key;
      if (vars) for (const [k, v] of Object.entries(vars)) msg = msg.replaceAll(`{${k}}`, String(v));
      return msg;
    },
    [locale],
  );

  const value = useMemo(() => ({ languageSetting, locale, setLocale, t }), [languageSetting, locale, setLocale, t]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18n {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within I18nProvider");
  return ctx;
}
