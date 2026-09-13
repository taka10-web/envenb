import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, queryKeys } from "./api";
import type { Settings } from "./types";

// Theme handling. The persisted value lives in the shared settings (so the
// CLI sees the same choice); localStorage only caches the last value to avoid
// a light→dark flash before the settings query resolves.

export type ThemeSetting = Settings["theme"];
export type ResolvedTheme = "light" | "dark";

const STORAGE_KEY = "envfish.theme";
const MEDIA = "(prefers-color-scheme: dark)";

function readCached(): ThemeSetting {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "system") return stored;
  } catch {
    /* storage unavailable */
  }
  return "system";
}

function writeCached(theme: ThemeSetting) {
  try {
    window.localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    /* ignore */
  }
}

function systemPrefersDark(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia(MEDIA).matches;
}

function applyClass(resolved: ResolvedTheme) {
  document.documentElement.classList.toggle("dark", resolved === "dark");
}

interface ThemeContextValue {
  /** The user's stored preference. */
  theme: ThemeSetting;
  /** What is actually painted right now. */
  resolved: ResolvedTheme;
  setTheme: (theme: ThemeSetting) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const qc = useQueryClient();
  const [theme, setThemeState] = useState<ThemeSetting>(readCached);
  const [prefersDark, setPrefersDark] = useState<boolean>(systemPrefersDark);

  const settings = useQuery({ queryKey: queryKeys.settings, queryFn: api.getSettings });

  // Adopt the persisted value once settings arrive (or whenever they change).
  useEffect(() => {
    if (settings.data) {
      setThemeState(settings.data.theme);
      writeCached(settings.data.theme);
    }
  }, [settings.data]);

  // Follow the OS while in "system" mode.
  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const mq = window.matchMedia(MEDIA);
    const onChange = (e: MediaQueryListEvent) => setPrefersDark(e.matches);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  const resolved: ResolvedTheme = theme === "system" ? (prefersDark ? "dark" : "light") : theme;

  useEffect(() => {
    applyClass(resolved);
  }, [resolved]);

  const { mutate: persist } = useMutation({
    mutationFn: (t: ThemeSetting) => api.setTheme(t),
    onSuccess: (data) => qc.setQueryData(queryKeys.settings, data),
  });

  const setTheme = useCallback(
    (t: ThemeSetting) => {
      setThemeState(t);
      writeCached(t);
      persist(t);
    },
    [persist],
  );

  const value = useMemo(() => ({ theme, resolved, setTheme }), [theme, resolved, setTheme]);
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}

// Apply the cached class synchronously at module load so the first paint is
// already in the right theme.
applyClass(readCached() === "system" ? (systemPrefersDark() ? "dark" : "light") : (readCached() as ResolvedTheme));
