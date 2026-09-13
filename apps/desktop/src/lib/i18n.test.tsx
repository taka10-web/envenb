import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import type { Settings } from "./types";

const settings: Settings = { language: "ja", key_backend: "file", theme: "system" };
const mocks = vi.hoisted(() => ({
  getSettings: vi.fn(),
  setLanguage: vi.fn(),
}));
vi.mock("./api", () => ({
  api: { getSettings: mocks.getSettings, setLanguage: mocks.setLanguage },
  queryKeys: { settings: ["settings"] as const },
}));

import { I18nProvider, useI18n } from "./i18n";

function Probe() {
  const { t, locale } = useI18n();
  return (
    <div>
      <span data-testid="locale">{locale}</span>
      <span data-testid="plain">{t("nav.projects")}</span>
      <span data-testid="vars">{t("projects.created", { date: "2026-01-02" })}</span>
      <span data-testid="multi">{t("vars.import.report", { publicAdded: 1, secretAdded: 2, skipped: 3 })}</span>
    </div>
  );
}

function renderWith(language: Settings["language"]) {
  mocks.getSettings.mockResolvedValue({ ...settings, language });
  mocks.setLanguage.mockImplementation(async (l: Settings["language"]) => ({ ...settings, language: l }));
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <I18nProvider>
        <Probe />
      </I18nProvider>
    </QueryClientProvider>,
  );
}

describe("I18nProvider", () => {
  it("renders Japanese strings when settings say ja", async () => {
    renderWith("ja");
    await waitFor(() => expect(screen.getByTestId("locale")).toHaveTextContent("ja"));
    expect(screen.getByTestId("plain")).toHaveTextContent("プロジェクト");
    expect(screen.getByTestId("vars")).toHaveTextContent("作成日 2026-01-02");
    await waitFor(() => expect(document.documentElement.lang).toBe("ja"));
  });

  it("renders English strings when settings say en", async () => {
    renderWith("en");
    await waitFor(() => expect(screen.getByTestId("locale")).toHaveTextContent("en"));
    expect(screen.getByTestId("plain")).toHaveTextContent("Projects");
    expect(screen.getByTestId("vars")).toHaveTextContent("Created 2026-01-02");
  });

  it("interpolates every {var} placeholder", async () => {
    renderWith("en");
    await waitFor(() => expect(screen.getByTestId("locale")).toHaveTextContent("en"));
    expect(screen.getByTestId("multi")).toHaveTextContent("Imported 1 public and 2 secret variables · 3 skipped");
    expect(screen.getByTestId("multi").textContent).not.toContain("{");
  });
});
