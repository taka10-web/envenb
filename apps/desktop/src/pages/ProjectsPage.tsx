import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { Plus } from "lucide-react";
import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle, Goldfish, GoldfishInline, GoldfishLoader, Input, Label } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { PageHeader } from "../components/PageHeader";
import { ErrorNote } from "../components/ErrorNote";
import { useI18n } from "../lib/i18n";

export function ProjectsPage() {
  const { t, locale } = useI18n();
  const qc = useQueryClient();
  const projects = useQuery({ queryKey: queryKeys.projects, queryFn: api.listProjects });
  const status = useQuery({ queryKey: queryKeys.status, queryFn: api.status });

  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const create = useMutation({
    mutationFn: () => api.createProject(name.trim(), path.trim() || null),
    onSuccess: () => {
      setName("");
      setPath("");
      void qc.invalidateQueries({ queryKey: queryKeys.projects });
      void qc.invalidateQueries({ queryKey: queryKeys.status });
    },
  });

  return (
    <div className="p-8">
      <PageHeader title={t("projects.title")} description={t("projects.description")} />

      <form
        className="mb-8 flex flex-wrap items-end gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) create.mutate();
        }}
      >
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="project-name">{t("common.name")}</Label>
          <Input id="project-name" placeholder="my-app" value={name} onChange={(e) => setName(e.target.value)} className="w-56" />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="project-path">{t("projects.localPath")}</Label>
          <Input id="project-path" placeholder="/Users/you/works/my-app" value={path} onChange={(e) => setPath(e.target.value)} className="w-80" />
        </div>
        <Button type="submit" disabled={!name.trim() || create.isPending}>
          {create.isPending ? <GoldfishInline /> : <Plus className="h-4 w-4" />} {t("projects.add")}
        </Button>
      </form>
      {create.error && <ErrorNote error={create.error} />}
      {projects.error && <ErrorNote error={projects.error} />}

      {projects.isLoading && <GoldfishLoader label={t("common.loading")} className="py-16" />}

      {projects.data?.length === 0 && (
        <div className="flex flex-col items-center gap-2 py-16 text-center text-muted-foreground">
          <Goldfish variant="red" size={6} />
          <p className="text-sm">{t("projects.empty")}</p>
        </div>
      )}

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {projects.data?.map((p) => (
          <Link key={p.id} to={`/projects/${p.id}`} className="block rounded-lg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
            <Card className="h-full transition-colors hover:bg-accent/40">
              <CardHeader>
                <CardTitle>{p.name}</CardTitle>
                <CardDescription className="truncate font-mono text-xs">{p.local_path ?? t("projects.noPath")}</CardDescription>
              </CardHeader>
              <CardContent className="text-xs text-muted-foreground">
                {t("projects.created", { date: new Date(p.created_at).toLocaleDateString(locale) })}
              </CardContent>
            </Card>
          </Link>
        ))}
      </div>

      {status.data && (
        <p className="mt-10 text-xs text-muted-foreground">
          {t("projects.footer", { projects: status.data.project_count, secrets: status.data.secret_count, key: status.data.master_key_location })}
        </p>
      )}
    </div>
  );
}
