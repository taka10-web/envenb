import { Navigate, NavLink, Route, Routes } from "react-router-dom";
import { Activity, BookOpen, Bot, Folder, Plug, Settings } from "lucide-react";
import { cn, Goldfish } from "@envfish/ui";
import { ProjectsPage } from "./pages/ProjectsPage";
import { ProjectDetailPage } from "./pages/ProjectDetailPage";
import { ConnectionsPage } from "./pages/ConnectionsPage";
import { AiAccessPage } from "./pages/AiAccessPage";
import { ActivityPage } from "./pages/ActivityPage";
import { SettingsPage } from "./pages/SettingsPage";
import { GuidePage } from "./pages/GuidePage";
import { useI18n, type MessageKey } from "./lib/i18n";

const NAV: { to: string; label: MessageKey; icon: typeof Folder }[] = [
  { to: "/projects", label: "nav.projects", icon: Folder },
  { to: "/connections", label: "nav.connections", icon: Plug },
  { to: "/ai-access", label: "nav.aiAccess", icon: Bot },
  { to: "/activity", label: "nav.activity", icon: Activity },
];

function navClass({ isActive }: { isActive: boolean }) {
  return cn(
    "flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
    isActive ? "bg-accent text-accent-foreground font-medium" : "text-muted-foreground hover:bg-accent/60",
  );
}

export default function App() {
  const { t } = useI18n();
  return (
    <div className="flex h-screen">
      <aside className="flex w-56 shrink-0 flex-col border-r bg-sidebar">
        <div className="flex items-center gap-2 px-4 py-4">
          <Goldfish variant="red" size={2} />
          <span className="font-semibold tracking-tight">EnvFish</span>
        </div>
        <nav className="flex flex-col gap-1 px-2">
          {NAV.map(({ to, label, icon: Icon }) => (
            <NavLink key={to} to={to} className={navClass}>
              <Icon className="h-4 w-4" />
              <span className="flex-1">{t(label)}</span>
            </NavLink>
          ))}
        </nav>
        <div className="mt-auto flex flex-col gap-3 px-2 pb-4">
          <NavLink to="/guide" className={navClass}>
            <BookOpen className="h-4 w-4" />
            <span className="flex-1">{t("nav.guide")}</span>
          </NavLink>
          <NavLink to="/settings" className={navClass}>
            <Settings className="h-4 w-4" />
            <span className="flex-1">{t("nav.settings")}</span>
          </NavLink>
          <div className="px-2 text-xs text-muted-foreground">
            <p>{t("tagline.1")}</p>
            <p>{t("tagline.2")}</p>
          </div>
        </div>
      </aside>
      <main className="flex-1 overflow-y-auto">
        <Routes>
          <Route path="/" element={<Navigate to="/projects" replace />} />
          <Route path="/projects" element={<ProjectsPage />} />
          <Route path="/projects/:projectId/*" element={<ProjectDetailPage />} />
          <Route path="/connections" element={<ConnectionsPage />} />
          <Route path="/ai-access" element={<AiAccessPage />} />
          <Route path="/guide" element={<GuidePage />} />
          <Route path="/activity" element={<ActivityPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
}
