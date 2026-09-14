import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, FileDown, Plus, Trash2 } from "lucide-react";
import { Button, GoldfishInline, GoldfishLoader, Input } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import type { Project } from "../lib/types";
import { ErrorNote } from "../components/ErrorNote";
import { SectionLabel } from "../components/SectionLabel";
import { ListRow, RowActions } from "../components/Table";
import { useI18n } from "../lib/i18n";
import { useAppContext } from "../lib/context";
import { confirmAsync } from "../lib/confirm";

const PRESETS = ["development", "staging", "production"];

/** Project settings: name/path, environments, delete. */
export function ProjectDetailPage() {
  const { t, locale } = useI18n();
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
  const qc = useQueryClient();

  const projects = useQuery({ queryKey: queryKeys.projects, queryFn: api.listProjects });
  const project = projects.data?.find((p) => p.id === projectId);

  const remove = useMutation({
    mutationFn: () => api.deleteProject(projectId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.projects });
      void qc.invalidateQueries({ queryKey: queryKeys.status });
      navigate("/projects");
    },
  });

  if (projects.isLoading) return <GoldfishLoader label={t("common.loading")} className="py-24" />;
  if (!project) return <p className="text-sm text-muted-foreground">{t("projects.notFound")}</p>;

  return (
    <div>
      <div className="mb-5 flex h-8 items-center gap-3">
        <Button asChild variant="ghost" size="icon-sm" aria-label={t("nav.projects")}>
          <Link to="/projects">
            <ArrowLeft className="h-4 w-4" />
          </Link>
        </Button>
        <h1 className="font-pixel text-lg leading-none">{project.name}</h1>
        <span className="truncate font-mono text-xs text-muted-foreground">{project.local_path ?? t("projects.noPath")}</span>
      </div>
      {remove.error && <ErrorNote error={remove.error} />}

      <Environments project={project} />

      <div className="mt-10 flex items-center justify-between border-t border-border/60 pt-4">
        <span className="font-mono text-[11px] text-muted-foreground">{t("projects.created", { date: new Date(project.created_at).toLocaleDateString(locale) })}</span>
        <Button
          variant="outline"
          size="sm"
          className="text-destructive hover:text-destructive"
          disabled={remove.isPending}
          onClick={() => {
            void confirmAsync(t("projects.confirmDelete", { name: project.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
              if (ok) remove.mutate();
            });
          }}
        >
          <Trash2 className="h-3.5 w-3.5" /> {t("projects.danger")}
        </Button>
      </div>
    </div>
  );
}

function Environments({ project }: { project: Project }) {
  const { t } = useI18n();
  const qc = useQueryClient();
  const navigate = useNavigate();
  const { setProject, setEnvironment } = useAppContext();
  const envs = useQuery({ queryKey: queryKeys.environments(project.id), queryFn: () => api.listEnvironments(project.id) });
  const [name, setName] = useState("");

  const invalidate = () => qc.invalidateQueries({ queryKey: queryKeys.environments(project.id) });
  const create = useMutation({
    mutationFn: (n: string) => api.createEnvironment(project.id, n),
    onSuccess: () => {
      setName("");
      void invalidate();
    },
  });
  const remove = useMutation({ mutationFn: (id: string) => api.deleteEnvironment(id), onSuccess: () => void invalidate() });

  const existing = new Set(envs.data?.map((e) => e.name.toLowerCase()));
  const missingPresets = PRESETS.filter((p) => !existing.has(p));

  const openImport = (environmentId: string) => {
    setProject(project.id);
    setEnvironment(environmentId);
    navigate("/variables?import=1");
  };

  return (
    <section>
      <SectionLabel>{t("envs.title")}</SectionLabel>
      <form
        className="mb-2 flex flex-wrap items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) create.mutate(name.trim());
        }}
      >
        <Input aria-label={t("envs.placeholder")} placeholder={t("envs.placeholder")} value={name} onChange={(e) => setName(e.target.value)} className="h-8 w-56 font-mono text-xs" />
        <Button type="submit" size="sm" disabled={!name.trim() || create.isPending}>
          {create.isPending ? <GoldfishInline /> : <Plus className="h-3.5 w-3.5" />} {t("common.add")}
        </Button>
        {missingPresets.length > 0 && (
          <span className="ml-2 flex items-center gap-1 font-mono text-[11px] text-muted-foreground">
            {t("envs.quickAdd")}
            {missingPresets.map((p) => (
              <Button key={p} type="button" variant="ghost" size="sm" className="h-7 px-2 font-mono text-[11px]" onClick={() => create.mutate(p)}>
                {p}
              </Button>
            ))}
          </span>
        )}
      </form>
      {create.error && <ErrorNote error={create.error} />}
      {remove.error && <ErrorNote error={remove.error} />}
      {envs.error && <ErrorNote error={envs.error} />}

      {envs.isLoading && <GoldfishLoader label={t("common.loading")} className="py-10" />}
      {envs.data?.length === 0 && <p className="py-4 text-sm text-muted-foreground">{t("envs.empty")}</p>}

      {envs.data && envs.data.length > 0 && (
        <div className="-mx-2">
          {envs.data.map((env) => (
            <ListRow key={env.id}>
              <span className="flex-1 font-mono text-sm">{env.name}</span>
              <RowActions>
                <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => openImport(env.id)}>
                  <FileDown className="h-3.5 w-3.5" /> {t("vars.import.button")}
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t("envs.deleteAria", { name: env.name })}
                  onClick={() => {
                    void confirmAsync(t("envs.confirmDelete", { name: env.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
                      if (ok) remove.mutate(env.id);
                    });
                  }}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </RowActions>
            </ListRow>
          ))}
        </div>
      )}
    </section>
  );
}
