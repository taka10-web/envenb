import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, queryKeys } from "./api";
import type { Environment, Project } from "./types";

// Global "where am I" context: the project and environment chosen in the top
// bar. Pages read it instead of route params. The choice is cached in
// localStorage; ids that no longer exist fall back to the first available item.

const STORAGE_KEY = "envenb.context";

type Stored = { projectId: string | null; environmentId: string | null };

function readStored(): Stored {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<Stored>;
      return {
        projectId: typeof parsed.projectId === "string" ? parsed.projectId : null,
        environmentId: typeof parsed.environmentId === "string" ? parsed.environmentId : null,
      };
    }
  } catch {
    /* storage unavailable or corrupt */
  }
  return { projectId: null, environmentId: null };
}

function writeStored(value: Stored) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  } catch {
    /* ignore */
  }
}

/** The stored id if it still exists, otherwise the first item (or null). */
function resolve<T extends { id: string }>(wanted: string | null, list: T[] | undefined): T | null {
  if (!list) return null;
  return list.find((x) => x.id === wanted) ?? list[0] ?? null;
}

export interface AppContextValue {
  projectId: string | null;
  environmentId: string | null;
  project: Project | null;
  environment: Environment | null;
  projects: Project[];
  environments: Environment[];
  /** True until the project list (and the chosen project's environments) have loaded. */
  isLoading: boolean;
  setProject: (projectId: string) => void;
  setEnvironment: (environmentId: string) => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export function AppContextProvider({ children }: { children: ReactNode }) {
  const [stored, setStored] = useState<Stored>(readStored);

  const projects = useQuery({ queryKey: queryKeys.projects, queryFn: api.listProjects });
  const project = resolve(stored.projectId, projects.data);
  const projectId = project?.id ?? null;

  const environments = useQuery({
    queryKey: queryKeys.environments(projectId ?? ""),
    queryFn: () => api.listEnvironments(projectId ?? ""),
    enabled: !!projectId,
  });
  const environment = projectId ? resolve(stored.environmentId, environments.data) : null;
  const environmentId = environment?.id ?? null;

  const isLoading = projects.isLoading || (!!projectId && environments.isLoading);

  // Persist the *effective* selection so a stale id heals itself on next launch.
  useEffect(() => {
    if (isLoading) return;
    writeStored({ projectId, environmentId });
  }, [isLoading, projectId, environmentId]);

  const setProject = useCallback((id: string) => setStored({ projectId: id, environmentId: null }), []);
  const setEnvironment = useCallback((id: string) => setStored((s) => ({ ...s, environmentId: id })), []);

  const value = useMemo<AppContextValue>(
    () => ({
      projectId,
      environmentId,
      project,
      environment,
      projects: projects.data ?? [],
      environments: (projectId && environments.data) || [],
      isLoading,
      setProject,
      setEnvironment,
    }),
    [projectId, environmentId, project, environment, projects.data, environments.data, isLoading, setProject, setEnvironment],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useAppContext(): AppContextValue {
  const ctx = useContext(AppContext);
  if (!ctx) throw new Error("useAppContext must be used within AppContextProvider");
  return ctx;
}
