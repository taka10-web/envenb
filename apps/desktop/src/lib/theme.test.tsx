import { render, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import type { Settings } from "./types";

const base: Settings = { language: "en", key_backend: "file", theme: "system" };
const mocks = vi.hoisted(() => ({
  getSettings: vi.fn(),
  setTheme: vi.fn(),
}));
vi.mock("./api", () => ({
  api: { getSettings: mocks.getSettings, setTheme: mocks.setTheme },
  queryKeys: { settings: ["settings"] as const },
}));

import { ThemeProvider } from "./theme";

function renderWith(theme: Settings["theme"]) {
  mocks.getSettings.mockResolvedValue({ ...base, theme });
  mocks.setTheme.mockImplementation(async (t: Settings["theme"]) => ({ ...base, theme: t }));
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ThemeProvider>
        <div />
      </ThemeProvider>
    </QueryClientProvider>,
  );
}

describe("ThemeProvider", () => {
  it("adds the dark class when the stored theme is dark", async () => {
    renderWith("dark");
    await waitFor(() => expect(document.documentElement).toHaveClass("dark"));
  });

  it("removes the dark class when the stored theme is light", async () => {
    document.documentElement.classList.add("dark");
    renderWith("light");
    await waitFor(() => expect(document.documentElement).not.toHaveClass("dark"));
  });
});
