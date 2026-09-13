import { useEffect, useRef, useState, type ChangeEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams } from "react-router-dom";
import { ClipboardCopy, FileDown, FolderOpen, Lock, Plus, Trash2, X } from "lucide-react";
import { Badge, Button, cn, GoldfishInline, GoldfishLoader, Input, Label } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import type { DotenvPreview, ImportReport, Project, VariableKind } from "../lib/types";
import { ErrorNote } from "../components/ErrorNote";
import { Segmented } from "../components/Segmented";
import { useI18n } from "../lib/i18n";
import { confirmAsync } from "../lib/confirm";

export function VariablesPage({ project }: { project: Project }) {
  const { t } = useI18n();
  const { environmentId } = useParams();
  const navigate = useNavigate();
  const envs = useQuery({ queryKey: queryKeys.environments(project.id), queryFn: () => api.listEnvironments(project.id) });

  // Default to the first environment when none is chosen.
  const selected = environmentId ?? envs.data?.[0]?.id;

  if (envs.isLoading) return <GoldfishLoader label={t("common.loading")} className="py-16" />;
  if (!envs.data?.length) return <p className="py-8 text-sm text-muted-foreground">{t("vars.createEnvFirst")}</p>;

  return (
    <div>
      <div className="mb-5 flex flex-wrap gap-1">
        {envs.data.map((env) => (
          <Button
            key={env.id}
            size="sm"
            variant={env.id === selected ? "default" : "outline"}
            onClick={() => navigate(`/projects/${project.id}/variables/${env.id}`)}
          >
            {env.name}
          </Button>
        ))}
      </div>
      {selected && <VariableTable environmentId={selected} />}
    </div>
  );
}

function VariableTable({ environmentId }: { environmentId: string }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const vars = useQuery({ queryKey: queryKeys.variables(environmentId), queryFn: () => api.listVariables(environmentId) });

  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const [kind, setKind] = useState<VariableKind>("PUBLIC");

  const invalidate = () => {
    void qc.invalidateQueries({ queryKey: queryKeys.variables(environmentId) });
    void qc.invalidateQueries({ queryKey: queryKeys.status });
  };
  const save = useMutation({
    mutationFn: () =>
      kind === "SECRET"
        ? api.setSecretVariable(environmentId, name.trim(), value)
        : api.setPublicVariable(environmentId, name.trim(), value),
    onSuccess: () => {
      setName("");
      setValue("");
      invalidate();
    },
  });
  const remove = useMutation({ mutationFn: (n: string) => api.deleteVariable(environmentId, n), onSuccess: invalidate });

  const [importOpen, setImportOpen] = useState(false);
  const [copied, setCopied] = useState<"ok" | "fail" | null>(null);
  useEffect(() => {
    if (!copied) return;
    const id = window.setTimeout(() => setCopied(null), 2500);
    return () => window.clearTimeout(id);
  }, [copied]);
  const copyExample = useMutation({
    mutationFn: async () => {
      const text = await api.renderEnvExample(environmentId);
      try {
        await navigator.clipboard.writeText(text);
        setCopied("ok");
      } catch {
        setCopied("fail");
      }
    },
  });

  return (
    <div>
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Button type="button" variant={importOpen ? "secondary" : "outline"} size="sm" onClick={() => setImportOpen((o) => !o)}>
          {importOpen ? <X className="h-3.5 w-3.5" /> : <FileDown className="h-3.5 w-3.5" />} {t("vars.import.button")}
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => copyExample.mutate()} disabled={copyExample.isPending}>
          {copyExample.isPending ? <GoldfishInline /> : <ClipboardCopy className="h-3.5 w-3.5" />} {t("vars.copyExample")}
        </Button>
        {copied === "ok" && <span className="text-xs text-muted-foreground">{t("vars.copied")}</span>}
        {copied === "fail" && <span className="text-xs text-destructive">{t("vars.copyFailed")}</span>}
      </div>
      {copyExample.error && <ErrorNote error={copyExample.error} />}
      {importOpen && (
        <DotenvImportPanel
          environmentId={environmentId}
          onImported={() => {
            invalidate();
          }}
        />
      )}

      <form
        className="mb-6 flex flex-wrap items-end gap-3 rounded-lg border bg-card p-4"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim() && value) save.mutate();
        }}
      >
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="var-name">{t("common.name")}</Label>
          <Input id="var-name" placeholder="OPENAI_API_KEY" value={name} onChange={(e) => setName(e.target.value)} className="w-56 font-mono" autoCapitalize="characters" />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="var-value">{t("common.value")}</Label>
          <Input
            id="var-value"
            type={kind === "SECRET" ? "password" : "text"}
            autoComplete="off"
            placeholder={kind === "SECRET" ? t("vars.secretPlaceholder") : "http://localhost:3000"}
            value={value}
            onChange={(e) => setValue(e.target.value)}
            className="w-80 font-mono"
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>{t("common.kind")}</Label>
          <div className="flex rounded-md border p-0.5">
            {(["PUBLIC", "SECRET"] as VariableKind[]).map((k) => (
              <button
                key={k}
                type="button"
                onClick={() => setKind(k)}
                className={cn(
                  "rounded px-3 py-1 text-xs font-medium transition-colors",
                  kind === k ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-accent",
                )}
              >
                {k}
              </button>
            ))}
          </div>
        </div>
        <Button type="submit" disabled={!name.trim() || !value || save.isPending}>
          {save.isPending ? <GoldfishInline /> : <Plus className="h-4 w-4" />} {t("common.save")}
        </Button>
        {kind === "SECRET" && (
          <p className="basis-full text-xs text-muted-foreground">
            <Lock className="mr-1 inline h-3 w-3" />
            {t("vars.secretNote")}
          </p>
        )}
      </form>
      {save.error && <ErrorNote error={save.error} />}
      {remove.error && <ErrorNote error={remove.error} />}
      {vars.error && <ErrorNote error={vars.error} />}

      {vars.isLoading && <GoldfishLoader label={t("common.loading")} className="py-10" />}

      {vars.data?.length === 0 && <p className="py-6 text-sm text-muted-foreground">{t("vars.empty")}</p>}

      {vars.data && vars.data.length > 0 && (
        <table className="w-full text-sm">
          <thead className="text-left text-xs uppercase tracking-wide text-muted-foreground">
            <tr className="border-b">
              <th className="py-2 pr-4 font-medium">{t("common.name")}</th>
              <th className="py-2 pr-4 font-medium">{t("common.kind")}</th>
              <th className="py-2 pr-4 font-medium">{t("common.value")}</th>
              <th className="py-2 font-medium" />
            </tr>
          </thead>
          <tbody>
            {vars.data.map((v) => (
              <tr key={v.id} className="border-b last:border-0">
                <td className="py-2 pr-4 font-mono">{v.name}</td>
                <td className="py-2 pr-4">
                  <Badge variant={v.kind === "SECRET" ? "secret" : "public"}>{v.kind}</Badge>
                </td>
                <td className="py-2 pr-4 font-mono text-muted-foreground">
                  {v.kind === "SECRET" ? <span title={t("vars.encryptedAtRest")}>••••••••</span> : v.value}
                </td>
                <td className="py-2 text-right">
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={t("envs.deleteAria", { name: v.name })}
                    onClick={() => {
                      void confirmAsync(t("vars.confirmDelete", { name: v.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
 if (ok) remove.mutate(v.name);
 });
                    }}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// .env import: paste → preview (Rust parses and suggests a kind) → import.

type Row = { name: string; value: string; kind: VariableKind; suggestion: DotenvPreview["entries"][number]["suggestion"]; line: number };

function DotenvImportPanel({ environmentId, onImported }: { environmentId: string; onImported: () => void }) {
  const { t } = useI18n();
  const [text, setText] = useState("");
  const [rows, setRows] = useState<Row[] | null>(null);
  const [invalid, setInvalid] = useState<number[]>([]);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [fileName, setFileName] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  const preview = useMutation({
    mutationFn: (source: string) => api.previewDotenv(source),
    onSuccess: (p) => {
      setReport(null);
      setInvalid(p.invalid_lines);
      setRows(
        p.entries.map((e) => ({
          name: e.name,
          value: e.value,
          // REVIEW is ambiguous; default to the safe side.
          kind: e.suggestion === "PUBLIC" ? "PUBLIC" : "SECRET",
          suggestion: e.suggestion,
          line: e.line,
        })),
      );
    },
  });
  const doImport = useMutation({
    mutationFn: () => api.importVariables(environmentId, (rows ?? []).map(({ name, value, kind }) => ({ name, value, kind }))),
    onSuccess: (r) => {
      setReport(r);
      setRows(null);
      setText("");
      setFileName(null);
      onImported();
    },
  });

  const setKindAt = (i: number, kind: VariableKind) => setRows((rs) => rs?.map((r, j) => (j === i ? { ...r, kind } : r)) ?? null);

  // Pure web API: the file never leaves the webview; its text is only pasted into the textarea.
  const onFileChosen = async (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    const content = await file.text();
    setText(content);
    setFileName(file.name);
    if (content.trim()) preview.mutate(content);
  };

  return (
    <div className="mb-6 rounded-lg border bg-card p-4">
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="dotenv-text">{t("vars.import.paste")}</Label>
        <textarea
          id="dotenv-text"
          value={text}
          onChange={(e) => setText(e.target.value)}
          rows={6}
          spellCheck={false}
          placeholder={"DATABASE_URL=postgres://localhost/app\nOPENAI_API_KEY=sk-..."}
          className="w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
        <p className="text-xs text-muted-foreground">{t("vars.import.hint")}</p>
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <input ref={fileInput} type="file" accept=".env,.env.*,text/plain" hidden data-testid="dotenv-file" onChange={(e) => void onFileChosen(e)} />
        <Button type="button" size="sm" variant="outline" onClick={() => fileInput.current?.click()} disabled={preview.isPending}>
          <FolderOpen className="h-3.5 w-3.5" /> {t("vars.import.chooseFile")}
        </Button>
        {fileName && <span className="font-mono text-xs text-muted-foreground">{t("vars.import.fileLoaded", { name: fileName })}</span>}
        <Button type="button" size="sm" variant="secondary" onClick={() => preview.mutate(text)} disabled={!text.trim() || preview.isPending}>
          {preview.isPending && <GoldfishInline />} {t("vars.import.preview")}
        </Button>
        {rows && rows.length > 0 && (
          <Button type="button" size="sm" onClick={() => doImport.mutate()} disabled={doImport.isPending}>
            {doImport.isPending && <GoldfishInline />} {t("vars.import.confirm", { count: rows.length })}
          </Button>
        )}
      </div>
      {preview.error && <ErrorNote error={preview.error} />}
      {doImport.error && <ErrorNote error={doImport.error} />}

      {invalid.length > 0 && (
        <p className="mt-3 text-xs text-destructive">{t("vars.import.invalidLines", { lines: invalid.join(", ") })}</p>
      )}
      {rows?.length === 0 && <p className="mt-3 text-sm text-muted-foreground">{t("vars.import.nothing")}</p>}

      {rows && rows.length > 0 && (
        <table className="mt-4 w-full text-sm">
          <thead className="text-left text-xs uppercase tracking-wide text-muted-foreground">
            <tr className="border-b">
              <th className="py-2 pr-4 font-medium">{t("common.name")}</th>
              <th className="py-2 pr-4 font-medium">{t("common.value")}</th>
              <th className="py-2 pr-4 font-medium">{t("common.kind")}</th>
              <th className="py-2 font-medium">{t("vars.import.suggestion")}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr key={`${r.line}-${r.name}`} className="border-b last:border-0">
                <td className="py-2 pr-4 font-mono">{r.name}</td>
                <td className="max-w-xs truncate py-2 pr-4 font-mono text-xs text-muted-foreground">{r.kind === "PUBLIC" ? r.value : "••••••••"}</td>
                <td className="py-2 pr-4">
                  <Segmented
                    size="sm"
                    value={r.kind}
                    onChange={(k) => setKindAt(i, k)}
                    options={[
                      { value: "PUBLIC" as VariableKind, label: "PUBLIC", activeClass: "bg-emerald-600 text-white" },
                      { value: "SECRET" as VariableKind, label: "SECRET", activeClass: "bg-amber-600 text-white" },
                    ]}
                  />
                </td>
                <td className="py-2">
                  {r.suggestion === "REVIEW" ? (
                    <Badge variant="secret">{t("vars.import.review")}</Badge>
                  ) : (
                    <Badge variant={r.suggestion === "SECRET" ? "secret" : "public"}>{r.suggestion}</Badge>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {report && (
        <p className="mt-3 text-sm">
          {t("vars.import.report", { publicAdded: report.public_added, secretAdded: report.secret_added, skipped: report.skipped.length })}
          {report.skipped.length > 0 && <span className="ml-1 font-mono text-xs text-muted-foreground">({report.skipped.join(", ")})</span>}
        </p>
      )}
    </div>
  );
}
