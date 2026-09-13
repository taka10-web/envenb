import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { ClipboardCopy, FileDown, Lock, Plus, Trash2, X } from "lucide-react";
import { Badge, Button, cn, GoldfishInline, GoldfishLoader, Input, Label } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import type { Project, VariableKind } from "../lib/types";
import { ErrorNote } from "../components/ErrorNote";
import { DotenvImport } from "../components/DotenvImport";
import { useI18n } from "../lib/i18n";
import { confirmAsync } from "../lib/confirm";

export function VariablesPage({ project }: { project: Project }) {
  const { t } = useI18n();
  const { environmentId } = useParams();
  const navigate = useNavigate();
  const envs = useQuery({ queryKey: queryKeys.environments(project.id), queryFn: () => api.listEnvironments(project.id) });

  // Default to the first environment when none is chosen.
  const selected = environmentId ?? envs.data?.[0]?.id;
  const selectedEnv = envs.data?.find((e) => e.id === selected);

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
      {selected && <VariableTable key={selected} environmentId={selected} environmentName={selectedEnv?.name ?? ""} />}
    </div>
  );
}

function VariableTable({ environmentId, environmentName }: { environmentId: string; environmentName: string }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
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

  // null = automatic: the import flow is open while the environment is empty
  // (or when arriving via `?import=1`), and hidden once the user closes it.
  const [importOpen, setImportOpen] = useState<boolean | null>(searchParams.get("import") === "1" ? true : null);
  const isEmpty = vars.data?.length === 0;
  const showImport = importOpen ?? isEmpty;
  const closeImport = () => {
    setImportOpen(false);
    if (searchParams.has("import")) {
      const next = new URLSearchParams(searchParams);
      next.delete("import");
      setSearchParams(next, { replace: true });
    }
  };
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
      {isEmpty && showImport && <h2 className="mb-3 text-base font-semibold tracking-tight">{t("vars.import.emptyHeading")}</h2>}
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Button
          type="button"
          variant={showImport ? "secondary" : "outline"}
          size="sm"
          className={cn(!showImport && "border-primary text-primary hover:text-primary")}
          onClick={() => (showImport ? closeImport() : setImportOpen(true))}
        >
          {showImport ? <X className="h-3.5 w-3.5" /> : <FileDown className="h-3.5 w-3.5" />} {t("vars.import.button")}
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => copyExample.mutate()} disabled={copyExample.isPending}>
          {copyExample.isPending ? <GoldfishInline /> : <ClipboardCopy className="h-3.5 w-3.5" />} {t("vars.copyExample")}
        </Button>
        {copied === "ok" && <span className="text-xs text-muted-foreground">{t("vars.copied")}</span>}
        {copied === "fail" && <span className="text-xs text-destructive">{t("vars.copyFailed")}</span>}
      </div>
      {copyExample.error && <ErrorNote error={copyExample.error} />}
      {showImport && (
        <div className="mb-6">
          <DotenvImport
            environmentId={environmentId}
            environmentName={environmentName}
            existingNames={vars.data?.map((v) => v.name) ?? []}
            onImported={invalidate}
            onClose={closeImport}
          />
        </div>
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
