import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { NavLink, Navigate, Route, Routes, useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, Trash2 } from "lucide-react";
import { Button, cn, GoldfishLoader } from "@envfish/ui";
import { api, queryKeys } from "../lib/api";
import { EnvironmentsPage } from "./EnvironmentsPage";
import { VariablesPage } from "./VariablesPage";
import { ConnectionsPage } from "./ConnectionsPage";
import { CredentialsPage } from "./CredentialsPage";
import { AiAccessPage } from "./AiAccessPage";
import { ActivityPage } from "./ActivityPage";
import { ErrorNote } from "../components/ErrorNote";
import { useI18n, type MessageKey } from "../lib/i18n";
import { confirmAsync } from "../lib/confirm";

const TABS: { to: string; label: MessageKey }[] = [
  { to: "environments", label: "tabs.environments" },
  { to: "variables", label: "tabs.variables" },
  { to: "connections", label: "tabs.connections" },
  { to: "credentials", label: "tabs.credentials" },
  { to: "ai-access", label: "tabs.aiAccess" },
  { to: "activity", label: "tabs.activity" },
];

export function ProjectDetailPage() {
  const { t } = useI18n();
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
  const qc = useQueryClient();

  const projects = useQuery({ queryKey: queryKeys.projects, queryFn: api.listProjects });
  const project = projects.data?.find((p) => p.id === projectId);

  const remove = useMutation({
    mutationFn: () => api.deleteProject(projectId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: queryKeys.projects });
      navigate("/projects");
    },
  });

  if (projects.isLoading) return <GoldfishLoader label={t("common.loading")} className="py-24" />;
  if (!project) return <div className="p-8 text-sm text-muted-foreground">{t("projects.notFound")}</div>;

  return (
    <div className="p-8">
      <div className="mb-6 flex items-start justify-between gap-4">
        <div>
          <Button asChild variant="ghost" size="sm" className="-ml-2 mb-2 text-muted-foreground">
            <NavLink to="/projects">
              <ArrowLeft className="h-4 w-4" /> {t("nav.projects")}
            </NavLink>
          </Button>
          <h1 className="text-2xl font-semibold tracking-tight">{project.name}</h1>
          <p className="mt-1 font-mono text-xs text-muted-foreground">{project.local_path ?? t("projects.noPath")}</p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            void confirmAsync(t("projects.confirmDelete", { name: project.name }), { confirm: t("common.delete"), cancel: t("common.cancel") }).then((ok) => {
 if (ok) remove.mutate();
 });
          }}
          disabled={remove.isPending}
        >
          <Trash2 className="h-4 w-4" /> {t("common.delete")}
        </Button>
      </div>
      {remove.error && <ErrorNote error={remove.error} />}

      <nav className="mb-6 flex gap-1 border-b">
        {TABS.map((tab) => (
          <NavLink
            key={tab.to}
            to={`/projects/${project.id}/${tab.to}`}
            className={({ isActive }) =>
              cn(
                "-mb-px border-b-2 px-3 py-2 text-sm transition-colors",
                isActive ? "border-primary text-foreground font-medium" : "border-transparent text-muted-foreground hover:text-foreground",
              )
            }
          >
            {t(tab.label)}
          </NavLink>
        ))}
      </nav>

      <Routes>
        <Route index element={<Navigate to={`/projects/${project.id}/environments`} replace />} />
        <Route path="environments" element={<EnvironmentsPage project={project} />} />
        <Route path="variables/:environmentId?" element={<VariablesPage project={project} />} />
        <Route path="connections" element={<ConnectionsPage project={project} />} />
        <Route path="credentials/:environmentId?" element={<CredentialsPage project={project} />} />
        <Route path="ai-access" element={<AiAccessPage project={project} />} />
        <Route path="activity" element={<ActivityPage project={project} />} />
      </Routes>
    </div>
  );
}
