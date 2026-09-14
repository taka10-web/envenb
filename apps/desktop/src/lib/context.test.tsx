import { act, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import type { Environment, Project } from "./types";

const projects: Project[] = [
  { id: "p1", name: "my-app", local_path: null, created_at: "", updated_at: "" },
  { id: "p2", name: "other", local_path: null, created_at: "", updated_at: "" },
];
const envsOf: Record<string, Environment[]> = {
  p1: [
    { id: "e1", project_id: "p1", name: "development", created_at: "", updated_at: "" },
    { id: "e2", project_id: "p1", name: "production", created_at: "", updated_at: "" },
  ],
  p2: [{ id: "e3", project_id: "p2", name: "development", created_at: "", updated_at: "" }],
};

const mocks = vi.hoisted(() => ({ listProjects: vi.fn(), listEnvironments: vi.fn() }));
vi.mock("./api", () => ({
  api: mocks,
  queryKeys: {
    projects: ["projects"] as const,
    environments: (projectId: string) => ["environments", projectId] as const,
  },
}));

import { AppContextProvider, useAppContext } from "./context";

let latest: ReturnType<typeof useAppContext> | null = null;
function Probe() {
  const ctx = useAppContext();
  latest = ctx;
  return (
    <span data-testid="ctx">
      {ctx.isLoading ? "loading" : `${ctx.projectId ?? "-"}/${ctx.environmentId ?? "-"}`}
    </span>
  );
}

function renderCtx(list: Project[] = projects) {
  mocks.listProjects.mockResolvedValue(list);
  mocks.listEnvironments.mockImplementation(async (id: string) => envsOf[id] ?? []);
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <AppContextProvider>
        <Probe />
      </AppContextProvider>
    </QueryClientProvider>,
  );
}

describe("AppContextProvider", () => {
  it("defaults to the first project and environment and persists the choice", async () => {
    renderCtx();
    await waitFor(() => expect(screen.getByTestId("ctx")).toHaveTextContent("p1/e1"));
    expect(JSON.parse(window.localStorage.getItem("envfish.context")!)).toEqual({ projectId: "p1", environmentId: "e1" });
  });

  it("restores a stored selection and heals ids that no longer exist", async () => {
    window.localStorage.setItem("envfish.context", JSON.stringify({ projectId: "p2", environmentId: "gone" }));
    renderCtx();
    await waitFor(() => expect(screen.getByTestId("ctx")).toHaveTextContent("p2/e3"));
  });

  it("resets the environment when the project changes", async () => {
    renderCtx();
    await waitFor(() => expect(screen.getByTestId("ctx")).toHaveTextContent("p1/e1"));
    act(() => latest!.setEnvironment("e2"));
    await waitFor(() => expect(screen.getByTestId("ctx")).toHaveTextContent("p1/e2"));
    act(() => latest!.setProject("p2"));
    await waitFor(() => expect(screen.getByTestId("ctx")).toHaveTextContent("p2/e3"));
  });

  it("reports no project when none exists", async () => {
    renderCtx([]);
    await waitFor(() => expect(screen.getByTestId("ctx")).toHaveTextContent("-/-"));
    expect(mocks.listEnvironments).not.toHaveBeenCalled();
  });
});
