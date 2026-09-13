import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Goldfish } from "@envfish/ui";
import { PageHeader } from "../components/PageHeader";
import { useI18n } from "../lib/i18n";

// Long-form guide text lives here per locale rather than in the flat i18n
// dictionary; commands are shared and copyable.

type Step = { title: string; body: string; commands?: string[] };
type Section = { title: string; description: string; steps: Step[] };

const GUIDE: Record<"en" | "ja", { title: string; description: string; sections: Section[]; defaults: string[][] }> = {
  en: {
    title: "How to use EnvFish",
    description: "From registering a project to letting Claude Code call your services without ever seeing a key.",
    sections: [
      {
        title: "1. Register a project and its environments",
        description: "One project per client or product. Environments hold different values for development, staging and production.",
        steps: [
          { title: "In this app", body: "Projects → Add project, then open it and add environments (quick add: development / staging / production)." },
          {
            title: "From the terminal",
            body: "The CLI reads and writes the same data directory, so whatever you add here shows up there and vice versa.",
            commands: ["envfish project add my-app --path ~/works/my-app", "envfish use my-app", "envfish env development --create"],
          },
        ],
      },
      {
        title: "2. Store variables and secrets",
        description: "PUBLIC values are plain configuration an AI may see. SECRET values are encrypted before they reach SQLite and are never shown again.",
        steps: [
          { title: "In this app", body: "Open a project → Variables. Choose PUBLIC or SECRET per entry, or paste a whole .env with “Import .env” and confirm the suggested classification." },
          {
            title: "From the terminal",
            body: "Secrets are read from stdin so they never end up in shell history or `ps`.",
            commands: ["envfish var set APP_URL http://localhost:3000", "printf '%s' \"$SUPABASE_SERVICE_KEY\" | envfish var set-secret SUPABASE_KEY", "envfish import .env"],
          },
        ],
      },
      {
        title: "3. Run your own app with everything injected",
        description: "Decrypted values go into the child process only. Do not start an AI agent this way; give it the broker instead.",
        steps: [{ title: "Terminal", body: "ENVFISH_PROJECT and ENVFISH_ENVIRONMENT are set as well.", commands: ["envfish run pnpm dev"] }],
      },
      {
        title: "4. Describe the services an AI may use",
        description: "A connection is a base URL plus the name of the SECRET that authenticates it. The value stays in the vault.",
        steps: [
          { title: "In this app", body: "Connections → pick the environment, kind (generic_http / openai / supabase), name, URL and the credential from the list of SECRET names." },
          { title: "From the terminal", body: "", commands: ["envfish connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY", "envfish connection add openai --kind openai --secret OPENAI_API_KEY"] },
        ],
      },
      {
        title: "5. Connect Claude Code (MCP)",
        description: "EnvFish runs as an MCP server. The AI gets list_* tools, call_service and supabase_select — no tool returns a secret.",
        steps: [
          { title: "Register once", body: "Use a different --client name per tool (codex, cursor, …) so permissions and the audit log stay separate.", commands: ["claude mcp add envfish -- envfish mcp --client claude-code"] },
          { title: "Then ask Claude", body: "For example: “Using EnvFish, list the beans table in my-app / development.” Claude calls supabase_select; EnvFish injects the key and returns rows." },
        ],
      },
      {
        title: "6. Decide what the AI may do",
        description: "Every brokered call is checked against your rules. ASK pauses the AI until you approve it here (AI Access) or with `envfish ai approve <id>`.",
        steps: [
          { title: "In this app", body: "AI Access → pick client / project / environment / connection and set READ / WRITE / DELETE to ALLOW, ASK or DENY. Pending approvals appear at the top and refresh automatically." },
          { title: "From the terminal", body: "", commands: ["envfish ai check --client claude-code --connection supabase", "envfish ai permit WRITE ALLOW --client claude-code --connection supabase", "envfish ai approvals", "envfish activity"] },
        ],
      },
      {
        title: "7. Harden the vault",
        description: "By default the master key is a 0600 file next to the database. Move it into the OS keychain when you are ready.",
        steps: [{ title: "Terminal", body: "The key is copied, read back, and only then is the file removed. Secrets need no re-encryption.", commands: ["envfish vault key-backend keychain"] }],
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
    description: "案件の登録から、Claude Code に鍵を見せずにサービスを呼ばせるまでの流れです。",
    sections: [
      {
        title: "1. 案件と環境を登録する",
        description: "案件・プロダクトごとに 1 つのプロジェクト。環境ごとに development / staging / production の値を分けて持ちます。",
        steps: [
          { title: "このアプリで", body: "「プロジェクト」→ プロジェクトを追加 → 開いて環境を追加 (development / staging / production はクイック追加)。" },
          {
            title: "ターミナルで",
            body: "CLI とこのアプリは同じデータを読み書きするため、どちらで登録しても双方に反映されます。",
            commands: ["envfish project add my-app --path ~/works/my-app", "envfish use my-app", "envfish env development --create"],
          },
        ],
      },
      {
        title: "2. 変数と Secret を登録する",
        description: "PUBLIC は AI に見えてよい設定値。SECRET は SQLite に届く前に暗号化され、以後は表示されません。",
        steps: [
          { title: "このアプリで", body: "プロジェクト → 「変数」タブ。1 件ずつ PUBLIC / SECRET を選ぶか、「.env を取り込む」に貼り付けて分類の提案を確認します。" },
          {
            title: "ターミナルで",
            body: "Secret は stdin から読み取るため、シェル履歴や ps に残りません。",
            commands: ["envfish var set APP_URL http://localhost:3000", "printf '%s' \"$SUPABASE_SERVICE_KEY\" | envfish var set-secret SUPABASE_KEY", "envfish import .env"],
          },
        ],
      },
      {
        title: "3. 自分のアプリに注入して起動する",
        description: "復号した値は子プロセスにだけ渡ります。AI エージェント自体をこの方法で起こさず、AI には次の Broker 経由を使ってください。",
        steps: [{ title: "ターミナル", body: "ENVFISH_PROJECT と ENVFISH_ENVIRONMENT も渡されます。", commands: ["envfish run pnpm dev"] }],
      },
      {
        title: "4. AI に使わせるサービスを定義する",
        description: "接続 = ベース URL + 認証に使う SECRET の「名前」。値は Vault から出ません。",
        steps: [
          { title: "このアプリで", body: "「接続」→ 環境・種別 (generic_http / openai / supabase)・名前・URL を入力し、認証情報は SECRET 名の一覧から選びます。" },
          { title: "ターミナルで", body: "", commands: ["envfish connection add supabase --kind supabase --url https://xyz.supabase.co --secret SUPABASE_KEY", "envfish connection add openai --kind openai --secret OPENAI_API_KEY"] },
        ],
      },
      {
        title: "5. Claude Code をつなぐ (MCP)",
        description: "EnvFish は MCP サーバーとして動きます。AI に見えるのは list_* 系と call_service / supabase_select だけで、Secret を返すツールはありません。",
        steps: [
          { title: "一度だけ登録", body: "Codex や Cursor など別ツールは --client の名前を変えて登録すると、権限と監査ログが分かれます。", commands: ["claude mcp add envfish -- envfish mcp --client claude-code"] },
          { title: "Claude に頼む", body: "例:「EnvFish を使って my-app / development の beans テーブルを一覧して」。Claude は supabase_select を呼び、EnvFish が鍵を付けて結果だけを返します。" },
        ],
      },
      {
        title: "6. AI に許す操作を決める",
        description: "Broker 経由の呼び出しはすべてルールで判定されます。ASK の間 AI は待機し、この画面 (AI アクセス) か `envfish ai approve <id>` で承認します。",
        steps: [
          { title: "このアプリで", body: "「AI アクセス」→ クライアント / プロジェクト / 環境 / 接続を選び、READ / WRITE / DELETE ごとに ALLOW・ASK・DENY を設定。承認待ちは画面上部に自動更新で並びます。" },
          { title: "ターミナルで", body: "", commands: ["envfish ai check --client claude-code --connection supabase", "envfish ai permit WRITE ALLOW --client claude-code --connection supabase", "envfish ai approvals", "envfish activity"] },
        ],
      },
      {
        title: "7. Vault を固める",
        description: "既定ではマスターキーは DB の隣の 0600 ファイルです。準備ができたら OS のキーチェーンへ移します。",
        steps: [{ title: "ターミナル", body: "鍵を書き込んで読み戻せることを確認してからファイルを削除します。Secret の再暗号化は不要です。", commands: ["envfish vault key-backend keychain"] }],
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
    <div className="flex items-center gap-2 rounded-md border bg-muted/40 px-3 py-1.5">
      <code className="flex-1 overflow-x-auto whitespace-nowrap font-mono text-xs">{command}</code>
      <Button type="button" variant="ghost" size="icon" className="h-7 w-7 shrink-0" onClick={copy} aria-label={copied ? copiedLabel : "copy"}>
        {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
      </Button>
    </div>
  );
}

export function GuidePage() {
  const { locale, t } = useI18n();
  const guide = GUIDE[locale];
  return (
    <div className="p-8">
      <div className="flex items-start justify-between gap-4">
        <PageHeader title={guide.title} description={guide.description} />
        <div className="flex gap-2 pt-1">
          <Goldfish variant="red" size={3} />
          <Goldfish variant="nishiki" size={3} />
          <Goldfish variant="demekin" size={3} />
        </div>
      </div>

      <div className="flex flex-col gap-4">
        {guide.sections.map((section) => (
          <Card key={section.title}>
            <CardHeader>
              <CardTitle>{section.title}</CardTitle>
              <CardDescription>{section.description}</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
              {section.steps.map((step) => (
                <div key={step.title} className="flex flex-col gap-2">
                  <p className="text-sm">
                    <span className="font-medium">{step.title}</span>
                    {step.body && <span className="text-muted-foreground"> — {step.body}</span>}
                  </p>
                  {step.commands?.map((c) => <CommandLine key={c} command={c} copiedLabel={t("vars.copied")} />)}
                </div>
              ))}
            </CardContent>
          </Card>
        ))}

        <Card>
          <CardHeader>
            <CardTitle>{t("ai.legend.title")}</CardTitle>
            <CardDescription>{t("ai.legend.note")}</CardDescription>
          </CardHeader>
          <CardContent>
            <table className="text-sm">
              <thead className="text-left text-xs uppercase tracking-wide text-muted-foreground">
                <tr>
                  {guide.defaults[0].map((h) => (
                    <th key={h} className="py-1 pr-6 font-medium">{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {guide.defaults.slice(1).map((row) => (
                  <tr key={row[0]} className="border-t">
                    {row.map((cell, i) => (
                      <td key={i} className={i === 0 ? "py-1.5 pr-6" : "py-1.5 pr-6 font-mono text-xs"}>{cell}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
