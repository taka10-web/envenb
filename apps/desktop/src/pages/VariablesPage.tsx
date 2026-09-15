import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import { ClipboardCopy, FileDown, Lock, Plus, Trash2, X } from "lucide-react";
import { Badge, Button, MaikoInline, MaikoLoader, Input } from "@envenb/ui";
import { api, queryKeys } from "../lib/api";
import type { VariableKind } from "../lib/types";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { DotenvImport } from "../components/DotenvImport";
import { Segmented } from "../components/Segmented";
import { Sheet } from "../components/Sheet";
import { Field } from "../components/Field";
import { WithEnvironment } from "../components/NeedsContext";
import { RowActions, Table, Td, Th, Tr } from "../components/Table";
import { useI18n } from "../lib/i18n";
import { useAppContext } from "../lib/context";
import { confirmAsync } from "../lib/confirm";

export function VariablesPage() {
  const { t } = useI18n();
  return (
    <div className="flex min-h-full flex-col">
      <PageHeader title={t("vars.title")} context />
      <WithEnvironment>{({ environmentId }) => <VariableTable key={environmentId} environmentId={environmentId} />}</WithEnvironment>
    </div>
  );
}

function VariableTable({ environmentId }: { environmentId: string }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const { environment, projectId } = useAppContext();
  const [searchParams, setSearchParams] = useSearchParams();
  const vars = useQuery({ queryKey: queryKeys.variables(environmentId), queryFn: () => api.listVariables(environmentId) });

  const [addOpen, setAddOpen] = useState(false);
  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const [kind, setKind] = useState<VariableKind>("PUBLIC");

  const invalidate = () => {
    void qc.invalidateQueries({ queryKey: queryKeys.variables(environmentId) });
    void qc.invalidateQueries({ queryKey: queryKeys.status });
  };
  const save = useMutation({
    mutationFn: () =>
      kind === "SECRET" ? api.setSecretVariable(environmentId, name.trim(), value) : api.setPublicVariable(environmentId, name.trim(), value),
    onSuccess: () => {
      setName("");
      setValue("");
      setKind("PUBLIC");
      setAddOpen(false);
      invalidate();
    },
  });
  const changeKind = useMutation({
    mutationFn: ({ name, kind }: { name: string; kind: VariableKind }) => api.changeVariableKind(environmentId, name, kind),
    onSuccess: invalidate,
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
    <div className="flex min-h-full flex-col">
      {isEmpty && showImport && <h2 className="mb-3 text-sm font-medium">{t("vars.import.emptyHeading")}</h2>}
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Button type="button" size="sm" onClick={() => setAddOpen(true)}>
          <Plus className="h-3.5 w-3.5" /> {t("vars.add")}
        </Button>
        <Button type="button" variant={showImport ? "secondary" : "outline"} size="sm" onClick={() => (showImport ? closeImport() : setImportOpen(true))}>
          {showImport ? <X className="h-3.5 w-3.5" /> : <FileDown className="h-3.5 w-3.5" />} {t("vars.import.button")}
        </Button>
        <Button type="button" variant="ghost" size="sm" onClick={() => copyExample.mutate()} disabled={copyExample.isPending}>
          {copyExample.isPending ? <MaikoInline /> : <ClipboardCopy className="h-3.5 w-3.5" />} {t("vars.copyExample")}
        </Button>
        {copied === "ok" && <span className="font-mono text-[11px] text-muted-foreground">{t("vars.copied")}</span>}
        {copied === "fail" && <span className="font-mono text-[11px] text-destructive">{t("vars.copyFailed")}</span>}
      </div>
      {copyExample.error && <ErrorNote error={copyExample.error} />}
      {showImport && (
        <div className="mb-5">
          <DotenvImport
            projectId={projectId ?? undefined}
            environmentId={environmentId}
            environmentName={environment?.name ?? ""}
            existingNames={vars.data?.map((v) => v.name) ?? []}
            onImported={invalidate}
            onClose={closeImport}
          />
        </div>
      )}

      {changeKind.error && <ErrorNote error={changeKind.error} />}
      {remove.error && <ErrorNote error={remove.error} />}
      {vars.error && <ErrorNote error={vars.error} />}

      <Table>
        <thead>
          <tr>
            <Th className="w-[28%]">{t("common.name")}</Th>
            <Th className="w-24">{t("common.kind")}</Th>
            <Th>{t("common.value")}</Th>
            <Th className="w-10" />
          </tr>
        </thead>
        <tbody>
          {vars.data?.map((v) => (
            <Tr key={v.id}>
              <Td className="font-mono text-xs">{v.name}</Td>
              <Td>
                {v.kind === "PUBLIC" ? (
                  <button
                    type="button"
                    title={t("vars.makeSecretHint")}
                    className="rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    onClick={() => {
                      void confirmAsync(t("vars.confirmMakeSecret", { name: v.name }), {
                        confirm: t("vars.makeSecret"),
                        cancel: t("common.cancel"),
                      }).then((ok) => {
                        if (ok) changeKind.mutate({ name: v.name, kind: "SECRET" });
                      });
                    }}
                  >
                    <Badge variant="public" className="cursor-pointer hover:opacity-80">
                      {v.kind}
                    </Badge>
                  </button>
                ) : (
                  <Badge variant="secret" title={t("vars.secretIsPermanent")}>
                    {v.kind}
                  </Badge>
                )}
              </Td>
              <Td className="max-w-0 truncate font-mono text-xs text-muted-foreground">
                {v.kind === "SECRET" ? <span title={t("vars.encryptedAtRest")}>••••••••</span> : v.value}
              </Td>
              <Td>
                <RowActions>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={t("envs.deleteAria", { name: v.name })}
                    onClick={() => {
                      void confirmAsync(t("vars.confirmDelete", { name: v.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
                        if (ok) remove.mutate(v.name);
                      });
                    }}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </RowActions>
              </Td>
            </Tr>
          ))}
        </tbody>
      </Table>
      {vars.isLoading && <MaikoLoader label={t("common.loading")} className="py-10" />}
      {isEmpty && !showImport && <p className="py-6 text-center text-sm text-muted-foreground">{t("vars.empty")}</p>}

      <Sheet open={addOpen} title={t("vars.add")} onClose={() => setAddOpen(false)}>
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            if (name.trim() && value) save.mutate();
          }}
        >
          <Field label={t("common.name")} htmlFor="var-name">
            <Input
              id="var-name"
              placeholder={t("vars.namePlaceholder")}
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="font-mono text-xs"
              autoCapitalize="characters"
            />
          </Field>
          <Field label={t("common.kind")} htmlFor="var-kind">
            <Segmented
              value={kind}
              onChange={setKind}
              ariaLabel={t("common.kind")}
              options={[
                { value: "PUBLIC" as VariableKind, label: "PUBLIC", activeClass: "bg-emerald-600 text-white" },
                { value: "SECRET" as VariableKind, label: "SECRET", activeClass: "bg-amber-600 text-white" },
              ]}
            />
          </Field>
          <Field label={t("common.value")} htmlFor="var-value">
            <Input
              id="var-value"
              type={kind === "SECRET" ? "password" : "text"}
              autoComplete="off"
              placeholder={t(kind === "SECRET" ? "vars.secretPlaceholder" : "vars.publicPlaceholder")}
              value={value}
              onChange={(e) => setValue(e.target.value)}
              className="font-mono text-xs"
            />
          </Field>
          {kind === "SECRET" && (
            <p className="flex items-start gap-1.5 font-mono text-[11px] text-muted-foreground">
              <Lock className="mt-0.5 h-3 w-3 shrink-0" />
              {t("vars.secretNote")}
            </p>
          )}
          {save.error && <ErrorNote error={save.error} />}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" size="sm" onClick={() => setAddOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button type="submit" size="sm" disabled={!name.trim() || !value || save.isPending}>
              {save.isPending ? <MaikoInline size={1} /> : <Plus className="h-3.5 w-3.5" />} {t("common.add")}
            </Button>
          </div>
        </form>
      </Sheet>
    </div>
  );
}
