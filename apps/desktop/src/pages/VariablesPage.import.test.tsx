import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import type { DotenvPreview, Environment, ImportReport, Project, Settings } from "../lib/types";

const settings: Settings = { language: "en", key_backend: "file", theme: "light" };
const env: Environment = { id: "env-1", project_id: "proj-1", name: "development", created_at: "", updated_at: "" };
const project: Project = { id: "proj-1", name: "my-app", local_path: null, created_at: "", updated_at: "" };
const previewResult: DotenvPreview = {
  entries: [
    { name: "APP_URL", value: "https://example.com", suggestion: "PUBLIC", line: 1 },
    { name: "API_TOKEN", value: "tok_123", suggestion: "SECRET", line: 2 },
  ],
  invalid_lines: [],
};
const report: ImportReport = { public_added: 1, secret_added: 1, skipped: [] };

const mocks = vi.hoisted(() => ({
  getSettings: vi.fn(),
  setLanguage: vi.fn(),
  setTheme: vi.fn(),
  listEnvironments: vi.fn(),
  listVariables: vi.fn(),
  previewDotenv: vi.fn(),
  importVariables: vi.fn(),
}));
vi.mock("../lib/api", () => ({
  api: mocks,
  queryKeys: {
    status: ["status"] as const,
    settings: ["settings"] as const,
    environments: (projectId: string) => ["environments", projectId] as const,
    variables: (environmentId: string) => ["variables", environmentId] as const,
  },
}));

import { I18nProvider } from "../lib/i18n";
import { ThemeProvider } from "../lib/theme";
import { VariablesPage } from "./VariablesPage";

function renderPage() {
  mocks.getSettings.mockResolvedValue(settings);
  mocks.setLanguage.mockResolvedValue(settings);
  mocks.setTheme.mockResolvedValue(settings);
  mocks.listEnvironments.mockResolvedValue([env]);
  mocks.listVariables.mockResolvedValue([]);
  mocks.previewDotenv.mockResolvedValue(previewResult);
  mocks.importVariables.mockResolvedValue(report);
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ThemeProvider>
        <I18nProvider>
          <MemoryRouter initialEntries={["/projects/proj-1/variables"]}>
            <VariablesPage project={project} />
          </MemoryRouter>
        </I18nProvider>
      </ThemeProvider>
    </QueryClientProvider>,
  );
}

describe("VariablesPage .env import", () => {
  it("previews pasted text, shows suggested kinds and imports with the chosen kinds", async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByRole("button", { name: /Import \.env/ });
    await user.click(screen.getByRole("button", { name: /Import \.env/ }));

    const textarea = await screen.findByLabelText("Paste .env content");
    await user.click(textarea);
    await user.paste("APP_URL=https://example.com\nAPI_TOKEN=tok_123");
    await user.click(screen.getByRole("button", { name: "Preview" }));

    await waitFor(() => expect(mocks.previewDotenv).toHaveBeenCalledWith("APP_URL=https://example.com\nAPI_TOKEN=tok_123"));

    const publicRow = (await screen.findByText("APP_URL")).closest("tr")!;
    const secretRow = screen.getByText("API_TOKEN").closest("tr")!;
    expect(within(publicRow).getByRole("button", { name: "PUBLIC" })).toHaveAttribute("aria-pressed", "true");
    expect(within(publicRow).getByText("https://example.com")).toBeInTheDocument();
    expect(within(secretRow).getByRole("button", { name: "SECRET" })).toHaveAttribute("aria-pressed", "true");
    expect(within(secretRow).queryByText("tok_123")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Import 2 variables" }));

    await screen.findByText("Imported 1 public and 1 secret variables · 0 skipped");
    expect(mocks.importVariables).toHaveBeenCalledTimes(1);
    expect(mocks.importVariables).toHaveBeenCalledWith("env-1", [
      { name: "APP_URL", value: "https://example.com", kind: "PUBLIC" },
      { name: "API_TOKEN", value: "tok_123", kind: "SECRET" },
    ]);
  });

  it("loads a chosen file into the textarea and previews it automatically", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Import \.env/ }));
    const file = new File(["APP_URL=https://example.com\nAPI_TOKEN=tok_123"], ".env.local", { type: "text/plain" });
    await user.upload(screen.getByTestId("dotenv-file"), file);

    await waitFor(() => expect(mocks.previewDotenv).toHaveBeenCalledWith("APP_URL=https://example.com\nAPI_TOKEN=tok_123"));
    expect(screen.getByLabelText("Paste .env content")).toHaveValue("APP_URL=https://example.com\nAPI_TOKEN=tok_123");
    expect(screen.getByText("Loaded .env.local")).toBeInTheDocument();
    await screen.findByText("API_TOKEN");
  });
});
