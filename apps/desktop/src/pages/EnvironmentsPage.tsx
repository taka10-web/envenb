import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { Plus, Trash2 } from "lucide-react";
import { Button, Card, CardContent, CardHeader, CardTitle, GoldfishLoader, Input } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import type { Project } from "../lib/types";
import { ErrorNote } from "../components/ErrorNote";
import { useI18n } from "../lib/i18n";
import { confirmAsync } from "../lib/confirm";

const PRESETS = ["development", "staging", "production"];

export function EnvironmentsPage({ project }: { project: Project }) {
  const { t } = useI18n();
  const qc = useQueryClient();
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

  return (
    <div>
      <form
        className="mb-4 flex items-center gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) create.mutate(name.trim());
        }}
      >
        <Input placeholder={t("envs.placeholder")} value={name} onChange={(e) => setName(e.target.value)} className="w-64" />
        <Button type="submit" disabled={!name.trim() || create.isPending}>
          <Plus className="h-4 w-4" /> {t("envs.add")}
        </Button>
        {missingPresets.length > 0 && (
          <div className="ml-2 flex items-center gap-1 text-xs text-muted-foreground">
            {t("envs.quickAdd")}
            {missingPresets.map((p) => (
              <Button key={p} type="button" variant="outline" size="sm" onClick={() => create.mutate(p)}>
                {p}
              </Button>
            ))}
          </div>
        )}
      </form>
      {create.error && <ErrorNote error={create.error} />}
      {remove.error && <ErrorNote error={remove.error} />}
      {envs.error && <ErrorNote error={envs.error} />}

      {envs.isLoading && <GoldfishLoader label={t("common.loading")} className="py-10" />}

      {envs.data?.length === 0 && <p className="py-8 text-sm text-muted-foreground">{t("envs.empty")}</p>}

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {envs.data?.map((env) => (
          <Card key={env.id}>
            <CardHeader className="flex-row items-center justify-between space-y-0">
              <CardTitle>
                <Link to={`../variables/${env.id}`} className="hover:underline">
                  {env.name}
                </Link>
              </CardTitle>
              <Button
                variant="ghost"
                size="icon"
                aria-label={t("envs.deleteAria", { name: env.name })}
                onClick={() => {
                  void confirmAsync(t("envs.confirmDelete", { name: env.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
 if (ok) remove.mutate(env.id);
 });
                }}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </CardHeader>
            <CardContent>
              <Button asChild variant="secondary" size="sm">
                <Link to={`../variables/${env.id}`}>{t("envs.open")}</Link>
              </Button>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
