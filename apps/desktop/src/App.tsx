import { Navigate, NavLink, Route, Routes } from "react-router-dom";
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

function tabClass({ isActive }: { isActive: boolean }) {
  return cn(
    "-mb-px flex h-9 items-center border-b-2 px-3 text-[13px] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
    isActive ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:text-foreground",
  );
}

const switcherClass =
  "h-7 max-w-[180px] truncate rounded-md border border-transparent bg-transparent pl-2 pr-6 font-mono text-xs text-foreground hover:border-border hover:bg-accent/60 focus-visible:border-border focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/40 disabled:opacity-60";

/** Project / environment switcher in the top bar. */
function ContextSwitcher() {
  const { t } = useI18n();
  const { projects, environments, projectId, environmentId, setProject, setEnvironment } = useAppContext();
  return (
    <div className="flex items-center gap-1 font-mono text-xs text-muted-foreground">
      <select aria-label={t("common.project")} className={switcherClass} value={projectId ?? ""} onChange={(e) => setProject(e.target.value)} disabled={projects.length === 0}>
        {projects.length === 0 && <option value="">{t("context.noProject")}</option>}
        {projects.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>
      <span aria-hidden>/</span>
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
  return (
    <div className="flex h-screen flex-col">
      <header className="flex h-12 shrink-0 items-center gap-6 border-b border-border px-4">
        <NavLink to="/variables" className="flex items-center gap-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
          <Goldfish variant="red" size={2} />
          <span className="font-pixel text-base leading-none">EnvFish</span>
        </NavLink>
        <ContextSwitcher />
      </header>
      <nav className="flex h-9 shrink-0 items-stretch border-b border-border px-4" aria-label="Pages">
        {NAV.map(({ to, label }) => (
          <NavLink key={to} to={to} className={tabClass}>
            {t(label)}
          </NavLink>
        ))}
      </nav>
      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-[1040px] px-6 py-6">
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
