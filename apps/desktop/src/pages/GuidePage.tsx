import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { Button } from "@envenb/ui";
import { PageHeader } from "../components/PageHeader";
import { SectionLabel } from "../components/SectionLabel";
import { useI18n } from "../lib/i18n";

// Long-form guide text lives here per locale rather than in the flat i18n
// dictionary; commands are shared and copyable.

/** A command line plus a one-line explanation of what it does. */
type Cmd = string | { run: string; note: string };
type Step = { title: string; body: string; commands?: Cmd[] };
type Section = { title: string; description: string; steps: Step[] };

/** Render `code` spans in body text; the guide is authored as plain prose. */
function renderBody(text: string) {
  return text.split(/(`[^`]+`)/).map((part, i) =>
    part.startsWith("`") && part.endsWith("`") && part.length > 2 ? (
      <code key={i} className="rounded bg-muted px-1 py-0.5 font-mono text-[0.85em]">
        {part.slice(1, -1)}
      </code>
    ) : (
      part
    ),
  );
}

/** "3. Store variables" → ["3", "Store variables"], so the data stays readable as prose. */
function splitNumber(title: string): [string, string] {
  const m = /^(\d+)\.\s*(.*)$/.exec(title);
  return m ? [m[1], m[2]] : ["", title];
}

const GUIDE: Record<"en" | "ja", { title: string; sections: Section[]; defaults: string[][] }> = {
  en: {
    title: "How to use EnvEnb",
    sections: [
      {
        title: "1. Install the CLI",
        description: "Once per machine. The desktop app and the CLI share the same vault, so either can be used.",
        steps: [
          {
            title: "From the repository",
            body: "This puts `envenb` in ~/.cargo/bin. Re-run with --force after pulling changes.",
            commands: [{ run: "pnpm install", note: "Installs the JavaScript dependencies of the desktop app." }, { run: "cargo install --path crates/cli --locked", note: "Builds the CLI and places the `envenb` binary in ~/.cargo/bin." }, { run: "envenb --version", note: "Checks that the command is on your PATH." }],
          },
          {
            title: "If the command is not found",
            body: "~/.cargo/bin is not on your PATH. Add this line to ~/.zshrc and open a new terminal.",
            commands: [{ run: "export PATH=\"$HOME/.cargo/bin:$PATH\"", note: "Adds Cargo's bin directory to your PATH. Put this line in ~/.zshrc." }],
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
            commands: [{ run: "envenb project add my-app --path ~/works/my-app", note: "Registers a project. --path is the local checkout, used by .gitignore and .env handling." }, { run: "envenb use my-app", note: "Makes it the current project for later commands." }, { run: "envenb env development --create", note: "Creates the environment if needed and selects it." }],
          },
        ],
      },
      {
        title: "3. Store variables and secrets",
        description: "PUBLIC values are plain configuration an AI may see. SECRET values are encrypted before they reach SQLite and are never shown again.",
        steps: [
          { title: "In this app", body: "Variables → “Add variable”, then choose PUBLIC or SECRET — or paste a whole .env with “Import .env” and confirm the suggested classification." },
          {
            title: "From the terminal",
            body: "Secrets are read from stdin so they never end up in shell history or `ps`.",
            commands: [{ run: "envenb var set APP_URL http://localhost:3000", note: "Stores a PUBLIC variable. The value is plain and an AI may read it." }, { run: "printf '%s' \"$SUPABASE_SERVICE_KEY\" | envenb var set-secret SUPABASE_KEY", note: "Stores a SECRET from stdin, so it never reaches your shell history or `ps`." }, { run: "envenb import .env", note: "Imports a whole .env, asking PUBLIC or SECRET per variable, and adds the file to .gitignore." }, { run: "envenb var copy SUPABASE_KEY", note: "Copies a SECRET to the clipboard when you need the value itself. Cleared after 30 seconds; never printed." }],
          },
        ],
      },
      {
        title: "4. Run your own app with everything injected",
        description: "Decrypted values go into the child process only. Do not start an AI agent this way; give it the broker instead.",
        steps: [{ title: "Terminal", body: "ENVENB_PROJECT and ENVENB_ENVIRONMENT are set as well.", commands: [{ run: "envenb run npm run dev", note: "Any command works: npm, python, go, docker, a shell script." }, { run: "envenb run python app.py", note: "The command is started with the variables and decrypted secrets in its environment." }] }],
      },
      {
        title: "5. Describe the services an AI may use",
        description: "A connection is a base URL plus the name of the SECRET that authenticates it. The value stays in the vault.",
        steps: [
          { title: "In this app", body: "Connections → Add connection: kind (generic_http / openai / supabase / …), name, URL and the credential from the list of SECRET names in the current environment." },
          { title: "From the terminal", body: "", commands: [{ run: "envenb connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY", note: "Defines a Supabase connection. --secret is the NAME of a stored SECRET, not its value." }, { run: "envenb connection add openai --kind openai --secret OPENAI_API_KEY", note: "Same for OpenAI; the base URL has a sensible default." }] },
        ],
      },
      {
        title: "6. Connect Claude Code (MCP)",
        description: "EnvEnb runs as an MCP server. The AI gets list_* tools, call_service and supabase_select — no tool returns a secret.",
        steps: [
          { title: "Register once", body: "Use a different --client name per tool (codex, cursor, …) so permissions and the audit log stay separate.", commands: [{ run: "claude mcp add envenb -- envenb mcp --client claude-code", note: "Registers EnvEnb as an MCP server in Claude Code, under the client name claude-code." }] },
          { title: "Then ask Claude", body: "For example: “Using EnvEnb, list the beans table in my-app / development.” Claude calls supabase_select; EnvEnb injects the key and returns rows." },
        ],
      },
      {
        title: "7. Decide what the AI may do",
        description: "Every brokered call is checked against your rules. ASK pauses the AI until you approve it here (AI Access) or with `envenb ai approve <id>`.",
        steps: [
          { title: "In this app", body: "AI Access → pick client / project / environment / connection and set READ / WRITE / DELETE to ALLOW, ASK or DENY. Pending approvals appear at the top and refresh automatically." },
          { title: "From the terminal", body: "", commands: [{ run: "envenb ai check --client claude-code --connection supabase", note: "Shows the effective READ / WRITE / DELETE decision for that client and connection." }, { run: "envenb ai permit WRITE ALLOW --client claude-code --connection supabase", note: "Writes a rule: this client may write to this connection without asking." }, { run: "envenb ai approvals", note: "Lists requests waiting for your decision, with their ids." }, { run: "envenb activity", note: "Shows the audit log: who called what, and whether it was allowed." }] },
        ],
      },
      {
        title: "8. Harden the vault",
        description: "By default the master key is a 0600 file next to the database. Move it into the OS keychain when you are ready.",
        steps: [{ title: "Terminal", body: "The key is copied, read back, and only then is the file removed. Secrets need no re-encryption.", commands: [{ run: "envenb vault key-backend keychain", note: "Moves the master key from the 0600 file into the OS keychain." }] }],
      },
      {
        title: "9. Credentials for humans",
        description: "Test accounts, SSH targets, database logins and certificates are structured secrets for you, not for the AI. Each copy button hands the value from Rust straight to the clipboard and clears it after 30 seconds.",
        steps: [
          { title: "In this app", body: "Credentials → Add credential: choose a kind (account / ssh / database / file) and fill in the fields. Secret fields are never shown again; use Copy, or Copy code for the current TOTP." },
          {
            title: "From the terminal",
            body: "The same records are available to the CLI, including SSH and wrapped commands that receive the credential as environment variables.",
            commands: [{ run: "envenb cred add my-account --kind account", note: "Adds a test account. Username, password and TOTP seed are prompted for, hidden." }, { run: "envenb cred copy my-account --field password", note: "Copies one field to the clipboard and clears it after 30 seconds." }, { run: "envenb ssh bastion", note: "Opens SSH with the stored key, written to a 0600 temp file and deleted on exit." }, { run: "envenb run --with-credentials -- sqlplus ...", note: "Like `envenb run`, but also passes credential fields as ENVENB_CRED_* variables." }],
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
    title: "EnvEnb の使い方",
    sections: [
      {
        title: "1. CLI をインストールする",
        description: "マシンごとに 1 回だけ。Desktop アプリと CLI は同じ Vault を読み書きするため、どちらからでも操作できます。",
        steps: [
          {
            title: "リポジトリから",
            body: "`envenb` が ~/.cargo/bin に入ります。更新時は --force を付けて入れ直してください。",
            commands: [{ run: "pnpm install", note: "Desktop アプリの JavaScript 依存関係を入れます。" }, { run: "cargo install --path crates/cli --locked", note: "CLI をビルドし、`envenb` を ~/.cargo/bin に置きます。" }, { run: "envenb --version", note: "PATH が通っているかの確認です。" }],
          },
          {
            title: "コマンドが見つからない場合",
            body: "~/.cargo/bin が PATH にありません。~/.zshrc に次の行を追記し、新しいターミナルを開いてください。",
            commands: [{ run: "export PATH=\"$HOME/.cargo/bin:$PATH\"", note: "Cargo の bin を PATH に追加します。~/.zshrc に書いてください。" }],
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
            commands: [{ run: "envenb project add my-app --path ~/works/my-app", note: "プロジェクトを登録します。--path はローカルの作業ディレクトリで、.gitignore や .env の処理に使われます。" }, { run: "envenb use my-app", note: "以降のコマンドの対象プロジェクトにします。" }, { run: "envenb env development --create", note: "環境が無ければ作成し、選択します。" }],
          },
        ],
      },
      {
        title: "3. 変数と Secret を登録する",
        description: "PUBLIC は AI に見えてよい設定値。SECRET は SQLite に届く前に暗号化され、以後は表示されません。",
        steps: [
          { title: "このアプリで", body: "「変数」→「変数を追加」で PUBLIC / SECRET を選んで入力するか、「.env を取り込む」に貼り付けて分類の提案を確認します。" },
          {
            title: "ターミナルで",
            body: "Secret は stdin から読み取るため、シェル履歴や ps に残りません。",
            commands: [{ run: "envenb var set APP_URL http://localhost:3000", note: "PUBLIC 変数を保存します。値は平文で、AI からも読めます。" }, { run: "printf '%s' \"$SUPABASE_SERVICE_KEY\" | envenb var set-secret SUPABASE_KEY", note: "SECRET を標準入力から保存します。シェル履歴や `ps` に残りません。" }, { run: "envenb import .env", note: ".env をまとめて取り込みます。変数ごとに PUBLIC / SECRET を確認し、ファイルを .gitignore に追記します。" }, { run: "envenb var copy SUPABASE_KEY", note: "値そのものが必要なときに、SECRET をクリップボードへコピーします。30 秒後に消去され、画面には出ません。" }],
          },
        ],
      },
      {
        title: "4. 自分のアプリに注入して起動する",
        description: "復号した値は子プロセスにだけ渡ります。AI エージェント自体をこの方法で起こさず、AI には次の Broker 経由を使ってください。",
        steps: [{ title: "ターミナル", body: "ENVENB_PROJECT と ENVENB_ENVIRONMENT も渡されます。", commands: [{ run: "envenb run npm run dev", note: "任意のコマンドが使えます。npm / python / go / docker / シェルスクリプトなど。" }, { run: "envenb run python app.py", note: "変数と復号した Secret を環境変数に入れて、コマンドを起動します。" }] }],
      },
      {
        title: "5. AI に使わせるサービスを定義する",
        description: "接続 = ベース URL + 認証に使う SECRET の「名前」。値は Vault から出ません。",
        steps: [
          { title: "このアプリで", body: "「接続」→ 接続を追加: 種別 (generic_http / openai / supabase など)・名前・URL を入力し、認証情報は現在の環境の SECRET 名の一覧から選びます。" },
          { title: "ターミナルで", body: "", commands: [{ run: "envenb connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY", note: "Supabase 接続を定義します。--secret は保存済み SECRET の「名前」で、値ではありません。" }, { run: "envenb connection add openai --kind openai --secret OPENAI_API_KEY", note: "OpenAI も同様です。ベース URL には既定値があります。" }] },
        ],
      },
      {
        title: "6. Claude Code をつなぐ (MCP)",
        description: "EnvEnb は MCP サーバーとして動きます。AI に見えるのは list_* 系と call_service / supabase_select だけで、Secret を返すツールはありません。",
        steps: [
          { title: "一度だけ登録", body: "Codex や Cursor など別ツールは --client の名前を変えて登録すると、権限と監査ログが分かれます。", commands: [{ run: "claude mcp add envenb -- envenb mcp --client claude-code", note: "EnvEnb を Claude Code に MCP サーバーとして登録します。クライアント名は claude-code です。" }] },
          { title: "Claude に頼む", body: "例:「EnvEnb を使って my-app / development の beans テーブルを一覧して」。Claude は supabase_select を呼び、EnvEnb が鍵を付けて結果だけを返します。" },
        ],
      },
      {
        title: "7. AI に許す操作を決める",
        description: "Broker 経由の呼び出しはすべてルールで判定されます。ASK の間 AI は待機し、この画面 (AI アクセス) か `envenb ai approve <id>` で承認します。",
        steps: [
          { title: "このアプリで", body: "「AI アクセス」→ クライアント / プロジェクト / 環境 / 接続を選び、READ / WRITE / DELETE ごとに ALLOW・ASK・DENY を設定。承認待ちは画面上部に自動更新で並びます。" },
          { title: "ターミナルで", body: "", commands: [{ run: "envenb ai check --client claude-code --connection supabase", note: "そのクライアントと接続に対する READ / WRITE / DELETE の実効判定を表示します。" }, { run: "envenb ai permit WRITE ALLOW --client claude-code --connection supabase", note: "ルールを書きます。このクライアントは、この接続への書き込みを確認なしで行えます。" }, { run: "envenb ai approvals", note: "あなたの判断を待っている要求を id 付きで一覧します。" }, { run: "envenb activity", note: "監査ログです。誰が何を呼び、許可されたかが分かります。" }] },
        ],
      },
      {
        title: "8. Vault を固める",
        description: "既定ではマスターキーは DB の隣の 0600 ファイルです。準備ができたら OS のキーチェーンへ移します。",
        steps: [{ title: "ターミナル", body: "鍵を書き込んで読み戻せることを確認してからファイルを削除します。Secret の再暗号化は不要です。", commands: [{ run: "envenb vault key-backend keychain", note: "マスターキーを 0600 のファイルから OS のキーチェーンへ移します。" }] }],
      },
      {
        title: "9. 人が使う資格情報",
        description: "テストアカウント・SSH 接続先・DB ログイン・証明書は、AI ではなく人が使う構造化された Secret です。コピーボタンは Rust からクリップボードへ直接値を渡し、30 秒後に自動で消去します。",
        steps: [
          { title: "このアプリで", body: "「資格情報」→ 資格情報を追加: 種別 (account / ssh / database / file) を選んで項目を入力します。Secret 項目は以後表示されません。必要なときは「コピー」、TOTP は「コードをコピー」を使います。" },
          {
            title: "ターミナルで",
            body: "同じレコードを CLI からも使えます。SSH 接続や、資格情報を環境変数として受け取るコマンドの起動にも対応しています。",
            commands: [{ run: "envenb cred add my-account --kind account", note: "テストアカウントを追加します。ユーザー名・パスワード・TOTP シードは非表示で入力します。" }, { run: "envenb cred copy my-account --field password", note: "フィールドを 1 つクリップボードにコピーし、30 秒後に消去します。" }, { run: "envenb ssh bastion", note: "保存した鍵で SSH に接続します。鍵は 0600 の一時ファイルに書かれ、終了時に消えます。" }, { run: "envenb run --with-credentials -- sqlplus ...", note: "`envenb run` に加えて、資格情報を ENVENB_CRED_* として渡します。" }],
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
    <div className="flex h-9 items-center gap-2 rounded-md border border-border/60 bg-muted/40 pl-3 pr-1">
      <span className="select-none font-mono text-xs text-muted-foreground/60" aria-hidden>
        $
      </span>
      <code className="flex-1 overflow-x-auto whitespace-nowrap font-mono text-xs">{command}</code>
      <Button type="button" variant="ghost" size="icon-sm" className="shrink-0 text-muted-foreground" onClick={copy} aria-label={copied ? copiedLabel : "copy"}>
        {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
      </Button>
    </div>
  );
}

export function GuidePage() {
  const { locale, t } = useI18n();
  const guide = GUIDE[locale];
  const sections = guide.sections.map((section) => {
    const [number, title] = splitNumber(section.title);
    return { ...section, number, title, id: `step-${number || title}` };
  });

  return (
    <div className="flex flex-row-reverse justify-end gap-10">
      {/* The guide is long; a jump list keeps its shape visible. */}
      <nav className="sticky top-6 hidden h-fit w-44 shrink-0 flex-col gap-1 md:flex" aria-label={guide.title}>
        <SectionLabel>{t("guide.contents")}</SectionLabel>
        {sections.map((section) => (
          <a key={section.id} href={`#${section.id}`} className="flex gap-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground">
            <span className="w-4 shrink-0 text-right font-mono">{section.number}</span>
            <span className="leading-snug">{section.title}</span>
          </a>
        ))}
      </nav>

      <div className="min-w-0 max-w-2xl">
        <PageHeader title={guide.title} />

        <div className="flex flex-col gap-12">
          {sections.map((section) => (
            <section key={section.id} id={section.id} className="scroll-mt-6">
              <div className="flex items-baseline gap-3">
                <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-border font-mono text-xs text-muted-foreground" aria-hidden>
                  {section.number}
                </span>
                <h2 className="text-base leading-tight">{section.title}</h2>
              </div>
              <p className="mt-2 pl-9 text-sm leading-relaxed text-muted-foreground">{renderBody(section.description)}</p>

              <div className="mt-4 flex flex-col gap-5 pl-9">
                {section.steps.map((step) => (
                  <div key={step.title}>
                    <h3 className="text-[13px] font-medium">{step.title}</h3>
                    {step.body && <p className="mt-1 text-sm leading-relaxed text-muted-foreground">{renderBody(step.body)}</p>}
                    {step.commands && (
                      <div className="mt-2 flex flex-col gap-3">
                        {step.commands.map((c) => {
                          const run = typeof c === "string" ? c : c.run;
                          const note = typeof c === "string" ? null : c.note;
                          return (
                            <div key={run}>
                              <CommandLine command={run} copiedLabel={t("vars.copied")} />
                              {note && <p className="mt-1 pl-3 text-xs leading-relaxed text-muted-foreground">{renderBody(note)}</p>}
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </section>
          ))}

          <section className="scroll-mt-6">
            <h2 className="text-base leading-tight">{t("ai.legend.title")}</h2>
            <p className="mt-2 text-sm leading-relaxed text-muted-foreground">{t("ai.legend.note")}</p>
            <table className="mt-4 text-sm">
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
    </div>
  );
}
