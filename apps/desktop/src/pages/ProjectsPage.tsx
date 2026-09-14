import { useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearchParams } from "react-router-dom";
import { Plus, Settings2 } from "lucide-react";
import { Button, MaikoInline, MaikoLoader, Input } from "@envenb/ui";
import { api, queryKeys } from "../lib/api";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { EmptyState } from "../components/EmptyState";
import { RowActions, Table, Td, Th, Tr } from "../components/Table";
import { useI18n } from "../lib/i18n";
import { useAppContext } from "../lib/context";

export function ProjectsPage() {
  const { t, locale } = useI18n();
  const qc = useQueryClient();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const wantsNew = searchParams.get("new") === "1";
  const { setProject } = useAppContext();
  const projects = useQuery({ queryKey: queryKeys.projects, queryFn: api.listProjects });
  const status = useQuery({ queryKey: queryKeys.status, queryFn: api.status });
  const envCounts = useQueries({
    queries: (projects.data ?? []).map((p) => ({ queryKey: queryKeys.environments(p.id), queryFn: () => api.listEnvironments(p.id) })),
  });

  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const create = useMutation({
    mutationFn: () => api.createProject(name.trim(), path.trim() || null),
    onSuccess: (p) => {
      setName("");
      setPath("");
      setProject(p.id);
      void qc.invalidateQueries({ queryKey: queryKeys.projects });
      void qc.invalidateQueries({ queryKey: queryKeys.status });
    },
  });

  const open = (id: string) => {
    setProject(id);
    navigate("/variables");
  };

  return (
    <div>
      <PageHeader title={t("projects.title")} />

      <form
        className="mb-6 rounded-lg border border-border bg-card p-4"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) create.mutate();
        }}
      >
        <div className="mb-3">
          <h2 className="font-mono text-[11px] uppercase tracking-[0.12em] text-muted-foreground">{t("projects.newTitle")}</h2>
          <p className="mt-1 text-xs text-muted-foreground">{t("projects.newHint")}</p>
        </div>
        <div className="flex items-center gap-2">
        <Input
          aria-label={t("common.name")}
          placeholder="my-app"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="h-8 w-56"
          autoFocus={wantsNew}
        />
        <Input aria-label={t("projects.localPath")} placeholder="/Users/you/works/my-app" value={path} onChange={(e) => setPath(e.target.value)} className="h-8 flex-1 font-mono text-xs" />
        <Button type="submit" size="sm" disabled={!name.trim() || create.isPending}>
          {create.isPending ? <MaikoInline /> : <Plus className="h-3.5 w-3.5" />} {t("projects.create")}
        </Button>
        </div>
      </form>
      {create.error && <ErrorNote error={create.error} />}
      {projects.error && <ErrorNote error={projects.error} />}

      {projects.isLoading && <MaikoLoader label={t("common.loading")} className="py-16" />}
      {projects.data?.length === 0 && <EmptyState text={t("projects.empty")} />}

      {projects.data && projects.data.length > 0 && (
        <Table>
          <thead>
            <tr>
              <Th>{t("common.name")}</Th>
              <Th>{t("common.path")}</Th>
              <Th>{t("envs.title")}</Th>
              <Th>{t("common.created")}</Th>
              <Th />
            </tr>
          </thead>
          <tbody>
            {projects.data.map((p, i) => {
              const count = envCounts[i]?.data?.length;
              return (
                <Tr key={p.id} className="cursor-pointer" onClick={() => open(p.id)}>
                  <Td>
                    <button
                      type="button"
                      className="font-medium focus-visible:outline-none focus-visible:underline"
                      onClick={(e) => {
                        e.stopPropagation();
                        open(p.id);
                      }}
                    >
                      {p.name}
                    </button>
                  </Td>
                  <Td className="max-w-[360px] truncate font-mono text-xs text-muted-foreground">{p.local_path ?? t("projects.noPath")}</Td>
                  <Td className="font-mono text-xs text-muted-foreground">{count === undefined ? "…" : t("envs.count", { count })}</Td>
                  <Td className="font-mono text-xs text-muted-foreground">{new Date(p.created_at).toLocaleDateString(locale)}</Td>
                  <Td className="w-10">
                    <RowActions>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={t("projects.settingsAria", { name: p.name })}
                        onClick={(e) => {
                          e.stopPropagation();
                          navigate(`/projects/${p.id}`);
                        }}
                      >
                        <Settings2 className="h-3.5 w-3.5" />
                      </Button>
                    </RowActions>
                  </Td>
                </Tr>
              );
            })}
          </tbody>
        </Table>
      )}

      {status.data && (
        <p className="mt-8 font-mono text-[11px] text-muted-foreground">
          {t("projects.footer", { projects: status.data.project_count, secrets: status.data.secret_count, key: status.data.master_key_location })}
        </p>
      )}
    </div>
  );
}
