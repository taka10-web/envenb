import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import type { DotenvPreview, Environment, ImportReport, Project, Settings, Variable } from "../lib/types";

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

const SOURCE = "APP_URL=https://example.com\nAPI_TOKEN=tok_123";

function renderPage({ variables = [], route = "/projects/proj-1/variables" }: { variables?: Variable[]; route?: string } = {}) {
  mocks.getSettings.mockResolvedValue(settings);
  mocks.setLanguage.mockResolvedValue(settings);
  mocks.setTheme.mockResolvedValue(settings);
  mocks.listEnvironments.mockResolvedValue([env]);
  mocks.listVariables.mockResolvedValue(variables);
  mocks.previewDotenv.mockResolvedValue(previewResult);
  mocks.importVariables.mockResolvedValue(report);
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ThemeProvider>
        <I18nProvider>
          <MemoryRouter initialEntries={[route]}>
            <VariablesPage project={project} />
          </MemoryRouter>
        </I18nProvider>
      </ThemeProvider>
    </QueryClientProvider>,
  );
}

const existingVar: Variable = {
  id: "var-1",
  environment_id: "env-1",
  name: "APP_URL",
  kind: "PUBLIC",
  value: "https://old.example.com",
  created_at: "",
  updated_at: "",
};

describe("VariablesPage .env import", () => {
  it("opens step 1 by default for an empty environment and imports pasted text with the chosen kinds", async () => {
    const user = userEvent.setup();
    renderPage();

    // Empty environment: the guided flow is open without clicking anything.
    await screen.findByRole("heading", { name: "Start by importing your .env or add variables one by one" });
    expect(screen.getByTestId("dotenv-dropzone")).toBeInTheDocument();
    expect(screen.getByRole("listitem", { current: "step" })).toHaveTextContent("Load");

    await user.click(screen.getByRole("button", { name: "Paste instead" }));
    const textarea = await screen.findByLabelText("Paste .env content");
    await user.click(textarea);
    await user.paste(SOURCE);

    // Pasting auto-runs the preview and moves to step 2.
    await waitFor(() => expect(mocks.previewDotenv).toHaveBeenCalledWith(SOURCE));
    expect(await screen.findByRole("listitem", { current: "step" })).toHaveTextContent("Review");

    const publicRow = (await screen.findByText("APP_URL")).closest("tr")!;
    const secretRow = screen.getByText("API_TOKEN").closest("tr")!;
    expect(within(publicRow).getByRole("button", { name: "PUBLIC" })).toHaveAttribute("aria-pressed", "true");
    expect(within(publicRow).getByText("https://example.com")).toBeInTheDocument();
    expect(within(secretRow).getByRole("button", { name: "SECRET" })).toHaveAttribute("aria-pressed", "true");
    expect(within(secretRow).queryByText("tok_123")).not.toBeInTheDocument();

    // The eye toggle reveals the user's own pasted value.
    await user.click(within(secretRow).getByRole("button", { name: "Show value of API_TOKEN" }));
    expect(within(secretRow).getByText("tok_123")).toBeInTheDocument();

    expect(screen.getByText("1 public · 1 secret · 0 skipped")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Import 2 variables" }));

    await screen.findByText("Imported 1 public and 1 secret variables · 0 skipped");
    expect(screen.getByText("Add .env to .gitignore and consider deleting the file")).toBeInTheDocument();
    expect(mocks.importVariables).toHaveBeenCalledTimes(1);
    expect(mocks.importVariables).toHaveBeenCalledWith("env-1", [
      { name: "APP_URL", value: "https://example.com", kind: "PUBLIC" },
      { name: "API_TOKEN", value: "tok_123", kind: "SECRET" },
    ]);
  });

  it("keeps the flow behind the toolbar button when variables exist and flags rows that will overwrite", async () => {
    const user = userEvent.setup();
    renderPage({ variables: [existingVar] });

    await screen.findByText("https://old.example.com");
    expect(screen.queryByTestId("dotenv-dropzone")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Import \.env/ }));
    const file = new File([SOURCE], ".env.local", { type: "text/plain" });
    await user.upload(screen.getByTestId("dotenv-file"), file);

    await waitFor(() => expect(mocks.previewDotenv).toHaveBeenCalledWith(SOURCE));
    // Only the row whose name already exists in the environment is flagged.
    const flagged = await screen.findAllByText("will overwrite");
    expect(flagged).toHaveLength(1);
    expect(within(flagged[0]!.closest("tr")!).getByText("APP_URL")).toBeInTheDocument();
    expect(within(screen.getByText("API_TOKEN").closest("tr")!).queryByText("will overwrite")).not.toBeInTheDocument();
  });

  it("opens the flow from ?import=1 and shows the filename after a chosen file", async () => {
    const user = userEvent.setup();
    renderPage({ variables: [existingVar], route: "/projects/proj-1/variables/env-1?import=1" });

    const zone = await screen.findByTestId("dotenv-dropzone");
    const file = new File([SOURCE], ".env.local", { type: "text/plain" });
    await user.upload(screen.getByTestId("dotenv-file"), file);

    await waitFor(() => expect(mocks.previewDotenv).toHaveBeenCalledWith(SOURCE));
    await screen.findByText("API_TOKEN");
    expect(zone).not.toBeInTheDocument();
    expect(screen.getByText(/Loaded \.env\.local/)).toBeInTheDocument();

    // Back returns to step 1 and remembers nothing about the previous file.
    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(await screen.findByTestId("dotenv-dropzone")).toBeInTheDocument();
  });

  it("accepts a dropped file on the drop zone", async () => {
    renderPage();

    const zone = await screen.findByTestId("dotenv-dropzone");
    const file = new File([SOURCE], ".env", { type: "text/plain" });
    fireEvent.drop(zone, { dataTransfer: { files: [file], types: ["Files"] } });

    await waitFor(() => expect(mocks.previewDotenv).toHaveBeenCalledWith(SOURCE));
    await screen.findByText("API_TOKEN");
  });

  it("excludes rows whose skip checkbox is ticked from the import", async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByTestId("dotenv-dropzone");
    const file = new File([SOURCE], ".env", { type: "text/plain" });
    await user.upload(screen.getByTestId("dotenv-file"), file);
    await screen.findByText("API_TOKEN");

    await user.click(screen.getByRole("checkbox", { name: "Skip APP_URL" }));
    expect(screen.getByText("0 public · 1 secret · 1 skipped")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Import 1 variables" }));

    await waitFor(() => expect(mocks.importVariables).toHaveBeenCalledTimes(1));
    expect(mocks.importVariables).toHaveBeenCalledWith("env-1", [{ name: "API_TOKEN", value: "tok_123", kind: "SECRET" }]);
  });
});
