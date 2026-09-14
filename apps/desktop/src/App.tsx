import { Link, Navigate, NavLink, Route, Routes } from "react-router-dom";
import { Plus } from "lucide-react";
import { cn, Goldfish } from "@envfish/ui";
import { ProjectsPage } from "./pages/ProjectsPage";
import { ProjectDetailPage } from "./pages/ProjectDetailPage";
import { VariablesPage } from "./pages/VariablesPage";
import { ConnectionsPage } from "./pages/ConnectionsPage";
import { CredentialsPage } from "./pages/CredentialsPage";
import { AiAccessPage } from "./pages/AiAccessPage";
import { ActivityPage } from "./pages/ActivityPage";
import { SettingsPage } from "./pages/SettingsPage";
import { GuidePage } from "./pages/GuidePage";
import { useI18n, type MessageKey } from "./lib/i18n";
import { useAppContext } from "./lib/context";

// Daily-use pages first; project administration and settings after.
const NAV: { to: string; label: MessageKey }[] = [
  { to: "/variables", label: "nav.variables" },
  { to: "/credentials", label: "nav.credentials" },
  { to: "/connections", label: "nav.connections" },
  { to: "/ai-access", label: "nav.aiAccess" },
  { to: "/activity", label: "nav.activity" },
  { to: "/projects", label: "nav.projects" },
  { to: "/settings", label: "nav.settings" },
  { to: "/guide", label: "nav.guide" },
];

function navClass({ isActive }: { isActive: boolean }) {
  return cn(
    "flex h-8 items-center gap-2 rounded-md border-l-2 px-3 text-[13px] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
    isActive
      ? "border-primary bg-accent/70 text-foreground"
      : "border-transparent text-muted-foreground hover:bg-accent/40 hover:text-foreground",
  );
}

const switcherClass =
  "h-8 w-full truncate rounded-md border border-border bg-background px-2 font-mono text-xs text-foreground hover:bg-accent/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/40 disabled:opacity-60";

/** Project / environment switcher at the top of the sidebar. */
function ContextSwitcher() {
  const { t } = useI18n();
  const { projects, environments, projectId, environmentId, setProject, setEnvironment } = useAppContext();
  return (
    <div className="flex flex-col gap-1.5 px-3 pb-3">
      <div className="flex items-center justify-between">
        <label className="font-mono text-[10px] uppercase tracking-[0.12em] text-muted-foreground">{t("common.project")}</label>
        <Link
          to="/projects?new=1"
          aria-label={t("projects.create")}
          title={t("projects.create")}
          className="flex h-5 w-5 items-center justify-center rounded border border-border text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <Plus className="h-3 w-3" />
        </Link>
      </div>
      {projects.length === 0 ? (
        <Link
          to="/projects?new=1"
          className="flex h-8 items-center justify-center gap-1 rounded-md border border-dashed border-border text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          <Plus className="h-3 w-3" /> {t("projects.create")}
        </Link>
      ) : (
        <select aria-label={t("common.project")} className={switcherClass} value={projectId ?? ""} onChange={(e) => setProject(e.target.value)}>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      )}
      <label className="mt-1 font-mono text-[10px] uppercase tracking-[0.12em] text-muted-foreground">{t("common.environment")}</label>
      <select
        aria-label={t("common.environment")}
        className={switcherClass}
        value={environmentId ?? ""}
        onChange={(e) => setEnvironment(e.target.value)}
        disabled={environments.length === 0}
      >
        {environments.length === 0 && <option value="">{t("context.noEnvironment")}</option>}
        {environments.map((e) => (
          <option key={e.id} value={e.id}>
            {e.name}
          </option>
        ))}
      </select>
    </div>
  );
}

export default function App() {
  const { t } = useI18n();
  const primary = NAV.slice(0, 6);
  const secondary = NAV.slice(6);
  return (
    <div className="flex h-screen">
      <aside className="flex w-56 shrink-0 flex-col border-r border-border bg-background/60">
        <NavLink
          to="/variables"
          className="flex h-12 items-center gap-2 border-b border-border px-4 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <Goldfish variant="red" size={2} />
          <span className="font-pixel text-base leading-none">EnvFish</span>
        </NavLink>
        <div className="border-b border-border pt-3">
          <ContextSwitcher />
        </div>
        <nav className="flex flex-1 flex-col gap-0.5 px-2 py-3" aria-label="Pages">
          {primary.map(({ to, label }) => (
            <NavLink key={to} to={to} className={navClass}>
              {t(label)}
            </NavLink>
          ))}
          <div className="mt-auto flex flex-col gap-0.5 border-t border-border pt-3">
            {secondary.map(({ to, label }) => (
              <NavLink key={to} to={to} className={navClass}>
                {t(label)}
              </NavLink>
            ))}
          </div>
        </nav>
      </aside>
      <main className="flex-1 overflow-y-auto">
        {/* Centred column with real top margin, so content is not pinned to the
            window edge on a wide screen. */}
        <div className="mx-auto w-full max-w-[920px] px-10 pb-16 pt-10">
          <Routes>
            <Route path="/" element={<Navigate to="/variables" replace />} />
            <Route path="/variables" element={<VariablesPage />} />
            <Route path="/credentials" element={<CredentialsPage />} />
            <Route path="/connections" element={<ConnectionsPage />} />
            <Route path="/ai-access" element={<AiAccessPage />} />
            <Route path="/activity" element={<ActivityPage />} />
            <Route path="/projects" element={<ProjectsPage />} />
            <Route path="/projects/:projectId" element={<ProjectDetailPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/guide" element={<GuidePage />} />
            <Route path="*" element={<Navigate to="/variables" replace />} />
          </Routes>
        </div>
      </main>
    </div>
  );
}
