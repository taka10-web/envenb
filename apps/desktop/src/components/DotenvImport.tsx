import { useRef, useState, type ChangeEvent, type DragEvent } from "react";
import { useMutation } from "@tanstack/react-query";
import { AlertTriangle, ArrowLeft, Check, ClipboardPaste, Eye, EyeOff, FileDown, FolderOpen, Upload, X } from "lucide-react";
import { Badge, Button, cn, GoldfishInline, Label } from "@envfish/ui";
import { api } from "../lib/api";
import type { DotenvPreview, ImportReport, VariableKind } from "../lib/types";
import { ErrorNote } from "./ErrorNote";
import { Segmented } from "./Segmented";
import { useI18n, type MessageKey } from "../lib/i18n";

// ---------------------------------------------------------------------------
// Guided .env import: 1 Load (drop / choose / paste) → 2 Review (kinds, skips)
// → 3 Done (report + .gitignore reminder). The file is read with the plain web
// File API and never leaves the webview; nothing is stored until step 3.

type Suggestion = DotenvPreview["entries"][number]["suggestion"];
type Step = "load" | "review" | "done";

type Row = {
  name: string;
  value: string;
  kind: VariableKind;
  suggestion: Suggestion;
  line: number;
  skip: boolean;
  revealed: boolean;
};

const STEPS: { id: Step; label: MessageKey }[] = [
  { id: "load", label: "vars.import.step.load" },
  { id: "review", label: "vars.import.step.review" },
  { id: "done", label: "vars.import.step.import" },
];

export function DotenvImport({
  environmentId,
  environmentName,
  existingNames = [],
  onImported,
  onClose,
}: {
  environmentId: string;
  environmentName: string;
  /** Names already present in the environment; matching rows are flagged "will overwrite". */
  existingNames?: string[];
  onImported: (report: ImportReport) => void;
  onClose?: () => void;
}) {
  const { t } = useI18n();
  const [step, setStep] = useState<Step>("load");
  const [pasteOpen, setPasteOpen] = useState(false);
  const [text, setText] = useState("");
  const [fileName, setFileName] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [rows, setRows] = useState<Row[]>([]);
  const [invalid, setInvalid] = useState<number[]>([]);
  const [report, setReport] = useState<ImportReport | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const pasted = useRef(false);

  const existing = new Set(existingNames);

  const preview = useMutation({
    mutationFn: (source: string) => api.previewDotenv(source),
    onSuccess: (p) => {
      setInvalid(p.invalid_lines);
      setRows(
        p.entries.map((e) => ({
          name: e.name,
          value: e.value,
          // REVIEW is ambiguous; default to the safe side.
          kind: e.suggestion === "PUBLIC" ? "PUBLIC" : "SECRET",
          suggestion: e.suggestion,
          line: e.line,
          skip: false,
          revealed: false,
        })),
      );
      setStep("review");
    },
  });

  const selected = rows.filter((r) => !r.skip);
  const doImport = useMutation({
    mutationFn: () => api.importVariables(environmentId, selected.map(({ name, value, kind }) => ({ name, value, kind }))),
    onSuccess: (r) => {
      setReport(r);
      setStep("done");
      onImported(r);
    },
  });

  const reset = () => {
    setStep("load");
    setPasteOpen(false);
    setText("");
    setFileName(null);
    setRows([]);
    setInvalid([]);
    setReport(null);
    preview.reset();
    doImport.reset();
  };

  const loadFile = async (file: File) => {
    const content = await file.text();
    setText(content);
    setFileName(file.name);
    preview.mutate(content);
  };

  const onFileChosen = (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (file) void loadFile(file);
  };

  const onDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setDragOver(false);
    const file = e.dataTransfer?.files?.[0];
    if (file) void loadFile(file);
  };

  const updateRow = (i: number, patch: Partial<Row>) => setRows((rs) => rs.map((r, j) => (j === i ? { ...r, ...patch } : r)));

  const publicCount = selected.filter((r) => r.kind === "PUBLIC").length;
  const secretCount = selected.length - publicCount;
  const skippedCount = rows.length - selected.length;

  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
        <StepIndicator step={step} />
        <span className="text-xs text-muted-foreground">
          {t("vars.import.target", { name: environmentName })}
          {fileName && step !== "load" && <span className="ml-2 font-mono">· {t("vars.import.fileLoaded", { name: fileName })}</span>}
        </span>
      </div>

      {step === "load" && (
        <div>
          <div
            data-testid="dotenv-dropzone"
            onDragOver={(e) => {
              e.preventDefault();
              setDragOver(true);
            }}
            onDragLeave={() => setDragOver(false)}
            onDrop={onDrop}
            className={cn(
              "flex flex-col items-center gap-3 rounded-lg border-2 border-dashed px-6 py-10 text-center transition-colors",
              dragOver ? "border-primary bg-primary/5" : "border-border",
            )}
          >
            <Upload className="h-8 w-8 text-muted-foreground" />
            <p className="text-sm font-medium">{t("vars.import.dropTitle")}</p>
            <div className="flex flex-wrap items-center justify-center gap-2">
              <input ref={fileInput} type="file" accept=".env,.env.*,text/plain" hidden data-testid="dotenv-file" onChange={onFileChosen} />
              <Button type="button" size="sm" variant="outline" onClick={() => fileInput.current?.click()} disabled={preview.isPending}>
                {preview.isPending ? <GoldfishInline /> : <FolderOpen className="h-3.5 w-3.5" />} {t("vars.import.chooseFile")}
              </Button>
              <Button type="button" size="sm" variant={pasteOpen ? "secondary" : "ghost"} onClick={() => setPasteOpen((o) => !o)} aria-pressed={pasteOpen}>
                <ClipboardPaste className="h-3.5 w-3.5" /> {t("vars.import.pasteInstead")}
              </Button>
            </div>
            {fileName && <span className="font-mono text-xs text-muted-foreground">{t("vars.import.fileLoaded", { name: fileName })}</span>}
          </div>
          <p className="mt-2 text-xs text-muted-foreground">{t("vars.import.helper")}</p>

          {pasteOpen && (
            <div className="mt-3 flex flex-col gap-1.5">
              <Label htmlFor="dotenv-text">{t("vars.import.paste")}</Label>
              <textarea
                id="dotenv-text"
                value={text}
                onPaste={() => {
                  pasted.current = true;
                }}
                onChange={(e) => {
                  const v = e.target.value;
                  setText(v);
                  setFileName(null);
                  // Pasting jumps straight to review; typing waits for "Continue".
                  if (pasted.current) {
                    pasted.current = false;
                    if (v.trim()) preview.mutate(v);
                  }
                }}
                rows={6}
                spellCheck={false}
                placeholder={"DATABASE_URL=postgres://localhost/my-app\nOPENAI_API_KEY=sk-..."}
                className="w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              />
              <div>
                <Button type="button" size="sm" onClick={() => preview.mutate(text)} disabled={!text.trim() || preview.isPending}>
                  {preview.isPending && <GoldfishInline />} {t("vars.import.continue")}
                </Button>
              </div>
            </div>
          )}
          {preview.error && (
            <div className="mt-3">
              <ErrorNote error={preview.error} />
            </div>
          )}
          {onClose && (
            <div className="mt-3 flex justify-end">
              <Button type="button" size="sm" variant="ghost" onClick={onClose}>
                <X className="h-3.5 w-3.5" /> {t("vars.import.close")}
              </Button>
            </div>
          )}
        </div>
      )}

      {step === "review" && (
        <div>
          {invalid.length > 0 && <p className="mb-3 text-xs text-destructive">{t("vars.import.invalidLines", { lines: invalid.join(", ") })}</p>}
          {rows.length === 0 && <p className="mb-3 text-sm text-muted-foreground">{t("vars.import.nothing")}</p>}

          {rows.length > 0 && (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead className="text-left text-xs uppercase tracking-wide text-muted-foreground">
                  <tr className="border-b">
                    <th className="py-2 pr-3 font-medium">{t("vars.import.skip")}</th>
                    <th className="py-2 pr-4 font-medium">{t("common.name")}</th>
                    <th className="py-2 pr-4 font-medium">{t("common.value")}</th>
                    <th className="py-2 pr-4 font-medium">{t("common.kind")}</th>
                    <th className="py-2 font-medium">{t("vars.import.note")}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r, i) => {
                    const masked = r.suggestion !== "PUBLIC" && !r.revealed;
                    return (
                      <tr key={`${r.line}-${r.name}`} className={cn("border-b last:border-0", r.skip && "opacity-50")}>
                        <td className="py-2 pr-3">
                          <input
                            type="checkbox"
                            className="h-4 w-4 accent-primary"
                            checked={r.skip}
                            aria-label={t("vars.import.skipAria", { name: r.name })}
                            onChange={(e) => updateRow(i, { skip: e.target.checked })}
                          />
                        </td>
                        <td className="py-2 pr-4 font-mono">{r.name}</td>
                        <td className="py-2 pr-4 font-mono text-xs text-muted-foreground">
                          <span className="inline-flex max-w-xs items-center gap-1">
                            <span className="truncate">{masked ? "••••••••" : r.value}</span>
                            {r.suggestion !== "PUBLIC" && (
                              <button
                                type="button"
                                className="rounded p-0.5 hover:bg-accent"
                                aria-label={r.revealed ? t("vars.import.hideValue", { name: r.name }) : t("vars.import.showValue", { name: r.name })}
                                onClick={() => updateRow(i, { revealed: !r.revealed })}
                              >
                                {r.revealed ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                              </button>
                            )}
                          </span>
                        </td>
                        <td className="py-2 pr-4">
                          <Segmented
                            size="sm"
                            value={r.kind}
                            ariaLabel={t("vars.import.kindAria", { name: r.name })}
                            onChange={(k) => updateRow(i, { kind: k })}
                            options={[
                              { value: "PUBLIC" as VariableKind, label: "PUBLIC", activeClass: "bg-emerald-600 text-white" },
                              {
                                value: "SECRET" as VariableKind,
                                label: "SECRET",
                                activeClass: r.suggestion === "REVIEW" ? "bg-amber-500 text-white ring-2 ring-amber-300" : "bg-amber-600 text-white",
                              },
                            ]}
                          />
                        </td>
                        <td className="py-2">
                          <div className="flex flex-wrap items-center gap-1.5">
                            {r.suggestion === "REVIEW" && <Badge variant="secret">{t("vars.import.review")}</Badge>}
                            {existing.has(r.name) && <Badge variant="outline">{t("vars.import.overwrite")}</Badge>}
                          </div>
                          {r.suggestion === "REVIEW" && <p className="mt-1 text-xs text-muted-foreground">{t("vars.import.reviewHint")}</p>}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}

          {doImport.error && (
            <div className="mt-3">
              <ErrorNote error={doImport.error} />
            </div>
          )}

          <div className="mt-4 flex flex-wrap items-center justify-between gap-2">
            <span className="text-xs text-muted-foreground">{t("vars.import.summary", { publicCount, secretCount, skippedCount })}</span>
            <div className="flex gap-2">
              <Button type="button" size="sm" variant="outline" onClick={reset} disabled={doImport.isPending}>
                <ArrowLeft className="h-3.5 w-3.5" /> {t("vars.import.back")}
              </Button>
              <Button type="button" size="sm" onClick={() => doImport.mutate()} disabled={selected.length === 0 || doImport.isPending}>
                {doImport.isPending ? <GoldfishInline /> : <FileDown className="h-3.5 w-3.5" />} {t("vars.import.confirm", { count: selected.length })}
              </Button>
            </div>
          </div>
        </div>
      )}

      {step === "done" && report && (
        <div>
          <p className="flex items-center gap-2 text-sm font-medium">
            <Check className="h-4 w-4 text-emerald-600" /> {t("vars.import.doneTitle")}
          </p>
          <p className="mt-1 text-sm">
            {t("vars.import.report", { publicAdded: report.public_added, secretAdded: report.secret_added, skipped: report.skipped.length })}
          </p>
          {report.skipped.length > 0 && (
            <p className="mt-1 text-xs text-muted-foreground">
              {t("vars.import.skippedNames")} <span className="font-mono">{report.skipped.join(", ")}</span>
            </p>
          )}
          <div className="mt-4 flex gap-3 rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-600" />
            <div>
              <p className="font-medium">{t("vars.import.reminderTitle")}</p>
              <p className="text-xs text-muted-foreground">{t("vars.import.reminderBody")}</p>
            </div>
          </div>
          <div className="mt-4 flex gap-2">
            <Button type="button" size="sm" variant="outline" onClick={reset}>
              <Upload className="h-3.5 w-3.5" /> {t("vars.import.another")}
            </Button>
            {onClose && (
              <Button type="button" size="sm" variant="secondary" onClick={onClose}>
                {t("vars.import.close")}
              </Button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function StepIndicator({ step }: { step: Step }) {
  const { t } = useI18n();
  const current = STEPS.findIndex((s) => s.id === step);
  return (
    <ol className="flex items-center gap-2 text-xs" aria-label={t("vars.import.stepsAria")}>
      {STEPS.map((s, i) => {
        const done = i < current;
        const active = i === current;
        return (
          <li key={s.id} className="flex items-center gap-2" aria-current={active ? "step" : undefined}>
            <span
              className={cn(
                "inline-flex h-5 w-5 items-center justify-center rounded-full border text-[11px] font-semibold",
                active && "border-primary bg-primary text-primary-foreground",
                done && "border-emerald-600 bg-emerald-600 text-white",
                !active && !done && "border-border text-muted-foreground",
              )}
            >
              {done ? <Check className="h-3 w-3" /> : i + 1}
            </span>
            <span className={cn(active ? "font-medium text-foreground" : "text-muted-foreground")}>{t(s.label)}</span>
            {i < STEPS.length - 1 && <span className="text-muted-foreground">→</span>}
          </li>
        );
      })}
    </ol>
  );
}
