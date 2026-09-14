import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { Button } from "@envfish/ui";
import { PageHeader } from "../components/PageHeader";
import { SectionLabel } from "../components/SectionLabel";
import { useI18n } from "../lib/i18n";

// Long-form guide text lives here per locale rather than in the flat i18n
// dictionary; commands are shared and copyable.

type Step = { title: string; body: string; commands?: string[] };
type Section = { title: string; description: string; steps: Step[] };

const GUIDE: Record<"en" | "ja", { title: string; sections: Section[]; defaults: string[][] }> = {
  en: {
    title: "How to use EnvFish",
    sections: [
      {
        title: "1. Install the CLI",
        description: "Once per machine. The desktop app and the CLI share the same vault, so either can be used.",
        steps: [
          {
            title: "From the repository",
            body: "This puts `envfish` in ~/.cargo/bin. Re-run with --force after pulling changes.",
            commands: ["pnpm install", "cargo install --path crates/cli --locked", "envfish --version"],
          },
          {
            title: "If the command is not found",
            body: "~/.cargo/bin is not on your PATH. Add this line to ~/.zshrc and open a new terminal.",
            commands: ["export PATH=\"$HOME/.cargo/bin:$PATH\""],
          },
        ],
      },
      {
        title: "2. Register a project and its environments",
        description: "One project per client or product. Environments hold different values for development, staging and production.",
        steps: [
          { title: "In this app", body: "Projects → add a project, then open its settings (gear icon) and add environments (quick add: development / staging / production). The top bar switches between projects and environments." },
          {
            title: "From the terminal",
            body: "The CLI reads and writes the same data directory, so whatever you add here shows up there and vice versa.",
            commands: ["envfish project add my-app --path ~/works/my-app", "envfish use my-app", "envfish env development --create"],
          },
        ],
      },
      {
        title: "3. Store variables and secrets",
        description: "PUBLIC values are plain configuration an AI may see. SECRET values are encrypted before they reach SQLite and are never shown again.",
        steps: [
          { title: "In this app", body: "Variables → type into the first row and choose PUBLIC or SECRET, or paste a whole .env with “Import .env” and confirm the suggested classification." },
          {
            title: "From the terminal",
            body: "Secrets are read from stdin so they never end up in shell history or `ps`.",
            commands: ["envfish var set APP_URL http://localhost:3000", "printf '%s' \"$SUPABASE_SERVICE_KEY\" | envfish var set-secret SUPABASE_KEY", "envfish import .env"],
          },
        ],
      },
      {
        title: "4. Run your own app with everything injected",
        description: "Decrypted values go into the child process only. Do not start an AI agent this way; give it the broker instead.",
        steps: [{ title: "Terminal", body: "ENVFISH_PROJECT and ENVFISH_ENVIRONMENT are set as well.", commands: ["envfish run pnpm dev"] }],
      },
      {
        title: "5. Describe the services an AI may use",
        description: "A connection is a base URL plus the name of the SECRET that authenticates it. The value stays in the vault.",
        steps: [
          { title: "In this app", body: "Connections → Add connection: kind (generic_http / openai / supabase / …), name, URL and the credential from the list of SECRET names in the current environment." },
          { title: "From the terminal", body: "", commands: ["envfish connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY", "envfish connection add openai --kind openai --secret OPENAI_API_KEY"] },
        ],
      },
      {
        title: "6. Connect Claude Code (MCP)",
        description: "EnvFish runs as an MCP server. The AI gets list_* tools, call_service and supabase_select — no tool returns a secret.",
        steps: [
          { title: "Register once", body: "Use a different --client name per tool (codex, cursor, …) so permissions and the audit log stay separate.", commands: ["claude mcp add envfish -- envfish mcp --client claude-code"] },
          { title: "Then ask Claude", body: "For example: “Using EnvFish, list the beans table in my-app / development.” Claude calls supabase_select; EnvFish injects the key and returns rows." },
        ],
      },
      {
        title: "7. Decide what the AI may do",
        description: "Every brokered call is checked against your rules. ASK pauses the AI until you approve it here (AI Access) or with `envfish ai approve <id>`.",
        steps: [
          { title: "In this app", body: "AI Access → pick client / project / environment / connection and set READ / WRITE / DELETE to ALLOW, ASK or DENY. Pending approvals appear at the top and refresh automatically." },
          { title: "From the terminal", body: "", commands: ["envfish ai check --client claude-code --connection supabase", "envfish ai permit WRITE ALLOW --client claude-code --connection supabase", "envfish ai approvals", "envfish activity"] },
        ],
      },
      {
        title: "8. Harden the vault",
        description: "By default the master key is a 0600 file next to the database. Move it into the OS keychain when you are ready.",
        steps: [{ title: "Terminal", body: "The key is copied, read back, and only then is the file removed. Secrets need no re-encryption.", commands: ["envfish vault key-backend keychain"] }],
      },
      {
        title: "9. Credentials for humans",
        description: "Test accounts, SSH targets, database logins and certificates are structured secrets for you, not for the AI. Each copy button hands the value from Rust straight to the clipboard and clears it after 30 seconds.",
        steps: [
          { title: "In this app", body: "Credentials → Add credential: choose a kind (account / ssh / database / file) and fill in the fields. Secret fields are never shown again; use Copy, or Copy code for the current TOTP." },
          {
            title: "From the terminal",
            body: "The same records are available to the CLI, including SSH and wrapped commands that receive the credential as environment variables.",
            commands: ["envfish cred add my-account --kind account", "envfish cred copy my-account --field password", "envfish ssh bastion", "envfish run --with-credentials -- sqlplus ..."],
          },
          { title: "What the AI sees", body: "Only names and non-secret fields, through list_credentials. There is no tool that returns a password, key or file content." },
        ],
      },
    ],
    defaults: [
      ["Environment", "READ", "WRITE", "DELETE"],
      ["development / staging", "ALLOW", "ASK", "DENY"],
      ["production / prod / live", "ASK", "DENY", "DENY"],
    ],
  },
  ja: {
    title: "EnvFish の使い方",
    sections: [
      {
        title: "1. CLI をインストールする",
        description: "マシンごとに 1 回だけ。Desktop アプリと CLI は同じ Vault を読み書きするため、どちらからでも操作できます。",
        steps: [
          {
            title: "リポジトリから",
            body: "`envfish` が ~/.cargo/bin に入ります。更新時は --force を付けて入れ直してください。",
            commands: ["pnpm install", "cargo install --path crates/cli --locked", "envfish --version"],
          },
          {
            title: "コマンドが見つからない場合",
            body: "~/.cargo/bin が PATH にありません。~/.zshrc に次の行を追記し、新しいターミナルを開いてください。",
            commands: ["export PATH=\"$HOME/.cargo/bin:$PATH\""],
          },
        ],
      },
      {
        title: "2. 案件と環境を登録する",
        description: "案件・プロダクトごとに 1 つのプロジェクト。環境ごとに development / staging / production の値を分けて持ちます。",
        steps: [
          { title: "このアプリで", body: "「プロジェクト」→ プロジェクトを追加 → 歯車アイコンから設定を開いて環境を追加します (development / staging / production はクイック追加)。プロジェクトと環境の切り替えは上部バーで行います。" },
          {
            title: "ターミナルで",
            body: "CLI とこのアプリは同じデータを読み書きするため、どちらで登録しても双方に反映されます。",
            commands: ["envfish project add my-app --path ~/works/my-app", "envfish use my-app", "envfish env development --create"],
          },
        ],
      },
      {
        title: "3. 変数と Secret を登録する",
        description: "PUBLIC は AI に見えてよい設定値。SECRET は SQLite に届く前に暗号化され、以後は表示されません。",
        steps: [
          { title: "このアプリで", body: "「変数」→ 先頭行に入力して PUBLIC / SECRET を選ぶか、「.env を取り込む」に貼り付けて分類の提案を確認します。" },
          {
            title: "ターミナルで",
            body: "Secret は stdin から読み取るため、シェル履歴や ps に残りません。",
            commands: ["envfish var set APP_URL http://localhost:3000", "printf '%s' \"$SUPABASE_SERVICE_KEY\" | envfish var set-secret SUPABASE_KEY", "envfish import .env"],
          },
        ],
      },
      {
        title: "4. 自分のアプリに注入して起動する",
        description: "復号した値は子プロセスにだけ渡ります。AI エージェント自体をこの方法で起こさず、AI には次の Broker 経由を使ってください。",
        steps: [{ title: "ターミナル", body: "ENVFISH_PROJECT と ENVFISH_ENVIRONMENT も渡されます。", commands: ["envfish run pnpm dev"] }],
      },
      {
        title: "5. AI に使わせるサービスを定義する",
        description: "接続 = ベース URL + 認証に使う SECRET の「名前」。値は Vault から出ません。",
        steps: [
          { title: "このアプリで", body: "「接続」→ 接続を追加: 種別 (generic_http / openai / supabase など)・名前・URL を入力し、認証情報は現在の環境の SECRET 名の一覧から選びます。" },
          { title: "ターミナルで", body: "", commands: ["envfish connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY", "envfish connection add openai --kind openai --secret OPENAI_API_KEY"] },
        ],
      },
      {
        title: "6. Claude Code をつなぐ (MCP)",
        description: "EnvFish は MCP サーバーとして動きます。AI に見えるのは list_* 系と call_service / supabase_select だけで、Secret を返すツールはありません。",
        steps: [
          { title: "一度だけ登録", body: "Codex や Cursor など別ツールは --client の名前を変えて登録すると、権限と監査ログが分かれます。", commands: ["claude mcp add envfish -- envfish mcp --client claude-code"] },
          { title: "Claude に頼む", body: "例:「EnvFish を使って my-app / development の beans テーブルを一覧して」。Claude は supabase_select を呼び、EnvFish が鍵を付けて結果だけを返します。" },
        ],
      },
      {
        title: "7. AI に許す操作を決める",
        description: "Broker 経由の呼び出しはすべてルールで判定されます。ASK の間 AI は待機し、この画面 (AI アクセス) か `envfish ai approve <id>` で承認します。",
        steps: [
          { title: "このアプリで", body: "「AI アクセス」→ クライアント / プロジェクト / 環境 / 接続を選び、READ / WRITE / DELETE ごとに ALLOW・ASK・DENY を設定。承認待ちは画面上部に自動更新で並びます。" },
          { title: "ターミナルで", body: "", commands: ["envfish ai check --client claude-code --connection supabase", "envfish ai permit WRITE ALLOW --client claude-code --connection supabase", "envfish ai approvals", "envfish activity"] },
        ],
      },
      {
        title: "8. Vault を固める",
        description: "既定ではマスターキーは DB の隣の 0600 ファイルです。準備ができたら OS のキーチェーンへ移します。",
        steps: [{ title: "ターミナル", body: "鍵を書き込んで読み戻せることを確認してからファイルを削除します。Secret の再暗号化は不要です。", commands: ["envfish vault key-backend keychain"] }],
      },
      {
        title: "9. 人が使う資格情報",
        description: "テストアカウント・SSH 接続先・DB ログイン・証明書は、AI ではなく人が使う構造化された Secret です。コピーボタンは Rust からクリップボードへ直接値を渡し、30 秒後に自動で消去します。",
        steps: [
          { title: "このアプリで", body: "「資格情報」→ 資格情報を追加: 種別 (account / ssh / database / file) を選んで項目を入力します。Secret 項目は以後表示されません。必要なときは「コピー」、TOTP は「コードをコピー」を使います。" },
          {
            title: "ターミナルで",
            body: "同じレコードを CLI からも使えます。SSH 接続や、資格情報を環境変数として受け取るコマンドの起動にも対応しています。",
            commands: ["envfish cred add my-account --kind account", "envfish cred copy my-account --field password", "envfish ssh bastion", "envfish run --with-credentials -- sqlplus ..."],
          },
          { title: "AI に見えるもの", body: "list_credentials で見えるのは名前と非 Secret 項目だけです。パスワード・鍵・ファイル内容を返すツールは存在しません。" },
        ],
      },
    ],
    defaults: [
      ["環境", "READ", "WRITE", "DELETE"],
      ["development / staging", "ALLOW", "ASK", "DENY"],
      ["production / prod / live", "ASK", "DENY", "DENY"],
    ],
  },
};

function CommandLine({ command, copiedLabel }: { command: string; copiedLabel: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard unavailable */
    }
  };
  return (
    <div className="group flex h-8 items-center gap-2 rounded-md border border-border/60 bg-muted/40 pl-3 pr-1">
      <code className="flex-1 overflow-x-auto whitespace-nowrap font-mono text-xs">{command}</code>
      <Button type="button" variant="ghost" size="icon-sm" className="shrink-0 opacity-0 transition-opacity focus-visible:opacity-100 group-hover:opacity-100" onClick={copy} aria-label={copied ? copiedLabel : "copy"}>
        {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
      </Button>
    </div>
  );
}

export function GuidePage() {
  const { locale, t } = useI18n();
  const guide = GUIDE[locale];
  return (
    <div className="max-w-3xl">
      <PageHeader title={guide.title} />

      <div className="flex flex-col gap-10">
        {guide.sections.map((section) => (
          <section key={section.title}>
            <SectionLabel>{section.title}</SectionLabel>
            <p className="mb-3 text-sm">{section.description}</p>
            <div className="flex flex-col divide-y divide-border/60">
              {section.steps.map((step) => (
                <div key={step.title} className="grid gap-2 py-3 sm:grid-cols-[140px_1fr]">
                  <span className="font-mono text-xs text-muted-foreground">{step.title}</span>
                  <div className="flex flex-col gap-2">
                    {step.body && <p className="text-sm">{step.body}</p>}
                    {step.commands?.map((c) => <CommandLine key={c} command={c} copiedLabel={t("vars.copied")} />)}
                  </div>
                </div>
              ))}
            </div>
          </section>
        ))}

        <section>
          <SectionLabel>{t("ai.legend.title")}</SectionLabel>
          <p className="mb-3 text-sm">{t("ai.legend.note")}</p>
          <table className="text-sm">
            <thead>
              <tr>
                {guide.defaults[0].map((h) => (
                  <th key={h} className="h-8 border-b border-border/60 pr-6 text-left font-mono text-[11px] font-medium uppercase tracking-[0.12em] text-muted-foreground">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {guide.defaults.slice(1).map((row) => (
                <tr key={row[0]} className="h-9 border-b border-border/60 last:border-0">
                  {row.map((cell, i) => (
                    <td key={i} className={i === 0 ? "pr-6" : "pr-6 font-mono text-xs"}>
                      {cell}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      </div>
    </div>
  );
}
