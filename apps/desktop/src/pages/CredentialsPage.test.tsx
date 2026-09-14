import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import type { Credential, CredentialKind, Environment, FieldSpec, Project, Settings } from "../lib/types";

const settings: Settings = { language: "en", key_backend: "file", theme: "light" };
const env: Environment = { id: "env-1", project_id: "proj-1", name: "development", created_at: "", updated_at: "" };
const project: Project = { id: "proj-1", name: "my-app", local_path: null, created_at: "", updated_at: "" };

const spec = (name: string, o: Partial<FieldSpec> = {}): FieldSpec => ({ name, secret: false, required: false, multiline: false, ...o });
const specs: [CredentialKind, FieldSpec[]][] = [
  ["account", [spec("url"), spec("username", { secret: true, required: true }), spec("password", { secret: true, required: true }), spec("totp_secret", { secret: true })]],
  ["ssh", [spec("host", { required: true }), spec("port"), spec("user", { required: true }), spec("private_key", { secret: true, multiline: true }), spec("passphrase", { secret: true }), spec("password", { secret: true })]],
  ["database", [spec("engine"), spec("host", { required: true }), spec("port"), spec("database"), spec("username", { secret: true, required: true }), spec("password", { secret: true, required: true })]],
  ["file", [spec("filename", { required: true }), spec("content", { secret: true, required: true, multiline: true })]],
];

// The marker "pw-value" stands in for a real secret. The IPC contract nulls secret values,
// so a mock that leaks one must still never reach the DOM.
const credentials: Credential[] = [
  {
    id: "cred-1",
    project_id: "proj-1",
    environment_id: "env-1",
    kind: "account",
    name: "staff-login",
    note: null,
    fields: [
      { field: "url", secret: false, value: "https://example.com/login", present: true },
      { field: "username", secret: true, value: null, present: true },
      { field: "password", secret: true, value: null, present: true },
      { field: "totp_secret", secret: true, value: null, present: false },
    ],
    created_at: "",
    updated_at: "",
  },
  {
    id: "cred-2",
    project_id: "proj-1",
    environment_id: "env-1",
    kind: "ssh",
    name: "bastion",
    note: null,
    fields: [
      { field: "host", secret: false, value: "bastion.example.com", present: true },
      { field: "port", secret: false, value: "22", present: true },
      { field: "user", secret: false, value: "deploy", present: true },
      { field: "private_key", secret: true, value: null, present: false },
      { field: "passphrase", secret: true, value: null, present: false },
      { field: "password", secret: true, value: null, present: false },
    ],
    created_at: "",
    updated_at: "",
  },
];

const mocks = vi.hoisted(() => ({
  getSettings: vi.fn(),
  setLanguage: vi.fn(),
  setTheme: vi.fn(),
  listProjects: vi.fn(),
  listEnvironments: vi.fn(),
  credentialFieldSpecs: vi.fn(),
  listCredentials: vi.fn(),
  createCredential: vi.fn(),
  updateCredentialFields: vi.fn(),
  deleteCredential: vi.fn(),
  copyCredentialField: vi.fn(),
}));
vi.mock("../lib/api", () => ({
  api: mocks,
  queryKeys: {
    status: ["status"] as const,
    settings: ["settings"] as const,
    projects: ["projects"] as const,
    environments: (projectId: string) => ["environments", projectId] as const,
    credentials: (projectId: string) => ["credentials", projectId] as const,
    credentialSpecs: ["credential-specs"] as const,
  },
}));

import { I18nProvider } from "../lib/i18n";
import { ThemeProvider } from "../lib/theme";
import { AppContextProvider } from "../lib/context";
import { CredentialsPage } from "./CredentialsPage";

function renderPage({ projects = [project] }: { projects?: Project[] } = {}) {
  mocks.getSettings.mockResolvedValue(settings);
  mocks.setLanguage.mockResolvedValue(settings);
  mocks.setTheme.mockResolvedValue(settings);
  mocks.listProjects.mockResolvedValue(projects);
  mocks.listEnvironments.mockResolvedValue([env]);
  mocks.credentialFieldSpecs.mockResolvedValue(specs);
  mocks.listCredentials.mockResolvedValue(credentials);
  mocks.copyCredentialField.mockResolvedValue(30);
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ThemeProvider>
        <I18nProvider>
          <AppContextProvider>
            <MemoryRouter initialEntries={["/credentials"]}>
              <CredentialsPage />
            </MemoryRouter>
          </AppContextProvider>
        </I18nProvider>
      </ThemeProvider>
    </QueryClientProvider>,
  );
}

describe("CredentialsPage", () => {
  it("lists credentials by kind with non-secret fields and never renders a secret value", async () => {
    const { container } = renderPage();

    await screen.findByText("staff-login");
    expect(screen.getByText("bastion")).toBeInTheDocument();
    expect(screen.getByText(/bastion\.example\.com:22/)).toBeInTheDocument();
    expect(screen.getByText(/user deploy/)).toBeInTheDocument();
    expect(screen.getByText(/https:\/\/example\.com\/login/)).toBeInTheDocument();

    expect(container.textContent).not.toContain("pw-value");
    // Secret fields that are present are shown only as a mask with a copy button.
    expect(screen.getByRole("button", { name: "Copy Password of staff-login" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Copy Username of staff-login" })).toBeInTheDocument();
    // totp_secret is not present → no one-time code button.
    expect(screen.queryByRole("button", { name: /one-time code/ })).not.toBeInTheDocument();
    // ssh has no present secrets → no copy button for it.
    expect(screen.queryByRole("button", { name: /of bastion$/ })).not.toBeInTheDocument();
  });

  it("copies a secret field through Rust and shows the clipboard hint", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: "Copy Password of staff-login" }));

    await waitFor(() => expect(mocks.copyCredentialField).toHaveBeenCalledWith("cred-1", "password"));
    expect(await screen.findByText("Copied · clears in 30s")).toBeInTheDocument();
  });

  it("opens the add sheet and renders the account form from the field specs with password inputs for secret fields", async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByText("staff-login");
    expect(screen.queryByLabelText("Kind")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Add credential" }));
    expect(screen.getByRole("dialog", { name: "Add credential" })).toBeInTheDocument();
    expect(screen.getByLabelText("Kind")).toHaveValue("account");
    expect(screen.getByLabelText("URL")).toHaveAttribute("type", "text");
    expect(screen.getByLabelText(/^Username/)).toHaveAttribute("type", "password");
    expect(screen.getByLabelText(/^Password/)).toHaveAttribute("type", "password");
    expect(screen.getByLabelText(/^TOTP secret/)).toHaveAttribute("type", "password");
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();

    // Escape closes the sheet.
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("submits a new account credential once name and required fields are filled", async () => {
    const user = userEvent.setup();
    mocks.createCredential.mockResolvedValue(credentials[0]);
    renderPage();

    await screen.findByText("staff-login");
    await user.click(screen.getByRole("button", { name: "Add credential" }));
    await user.type(screen.getByLabelText("Name"), "my-account");
    await user.type(screen.getByLabelText("URL"), "https://example.com");
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    await user.type(screen.getByLabelText(/^Username/), "alice");
    await user.type(screen.getByLabelText(/^Password/), "pw-value");
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(mocks.createCredential).toHaveBeenCalledWith({
        environment_id: "env-1",
        kind: "account",
        name: "my-account",
        note: null,
        fields: [
          { field: "url", value: "https://example.com" },
          { field: "username", value: "alice" },
          { field: "password", value: "pw-value" },
        ],
      }),
    );
    // The sheet closes after a successful save.
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("switches the form fields when the kind changes and offers Load from file for multiline secrets", async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByText("staff-login");
    await user.click(screen.getByRole("button", { name: "Add credential" }));
    await user.selectOptions(screen.getByLabelText("Kind"), "ssh");
    expect(screen.getByLabelText(/^Host/)).toHaveAttribute("type", "text");
    expect(screen.getByLabelText(/^Private key/).tagName).toBe("TEXTAREA");
    expect(screen.getByRole("button", { name: "Load from file" })).toBeInTheDocument();

    const file = new File(["-----BEGIN KEY-----\nabc\n-----END KEY-----"], "id_ed25519", { type: "text/plain" });
    await user.upload(screen.getByTestId("cred-field-private_key-file"), file);
    await waitFor(() => expect(screen.getByLabelText(/^Private key/)).toHaveValue("-----BEGIN KEY-----\nabc\n-----END KEY-----"));
    expect(screen.getByText("Loaded id_ed25519")).toBeInTheDocument();
  });

  it("updates only the fields the user filled in, keeping stored secrets", async () => {
    const user = userEvent.setup();
    mocks.updateCredentialFields.mockResolvedValue(credentials[1]);
    renderPage();

    await user.click(await screen.findByRole("button", { name: "Update bastion" }));
    expect(screen.getByRole("dialog", { name: "Updating bastion" })).toBeInTheDocument();
    expect(screen.getByLabelText(/^Host/)).toHaveValue("bastion.example.com");
    expect(screen.getByLabelText(/^Passphrase/)).toHaveValue("");

    await user.clear(screen.getByLabelText(/^Port/));
    await user.type(screen.getByLabelText(/^Port/), "2222");
    await user.type(screen.getByLabelText(/^Passphrase/), "pw-value");
    await user.click(screen.getByRole("button", { name: "Update" }));

    await waitFor(() =>
      expect(mocks.updateCredentialFields).toHaveBeenCalledWith(
        "cred-2",
        [
          { field: "port", value: "2222" },
          { field: "passphrase", value: "pw-value" },
        ],
        null,
      ),
    );
  });

  it("deletes after confirmation", async () => {
    const user = userEvent.setup();
    mocks.deleteCredential.mockResolvedValue(null);
    renderPage();

    await user.click(await screen.findByRole("button", { name: "Delete bastion" }));
    // In-page confirmation dialog (Tauri's WebView has no native window.confirm).
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(mocks.deleteCredential).toHaveBeenCalledWith("cred-2"));
  });

  it("guides to project creation when no project exists", async () => {
    renderPage({ projects: [] });

    await screen.findByText("Create a project to get started.");
    expect(screen.getByRole("link", { name: "Create project" })).toHaveAttribute("href", "/projects?new=1");
    expect(screen.queryByRole("button", { name: "Add credential" })).not.toBeInTheDocument();
    expect(mocks.listCredentials).not.toHaveBeenCalled();
  });
});
