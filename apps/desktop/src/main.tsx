import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { HashRouter } from "react-router-dom";
import App from "./App";
import { I18nProvider } from "./lib/i18n";
import { ThemeProvider } from "./lib/theme";
import { AppContextProvider } from "./lib/context";
import "./index.css";

if (import.meta.env.DEV) {
  const report = (kind: string, detail: unknown) => {
    const text = detail instanceof Error ? `${detail.message}` : String(detail);
    void fetch("/__envenb_log", { method: "POST", body: `${kind}: ${text}` }).catch(() => {});
  };
  window.addEventListener("error", (e) => report("error", e.error ?? e.message));
  window.addEventListener("unhandledrejection", (e) => report("unhandledrejection", e.reason));
  const origError = console.error.bind(console);
  console.error = (...args: unknown[]) => {
    report("console.error", args.map((a) => (a instanceof Error ? a.message : typeof a === "string" ? a : JSON.stringify(a))).join(" "));
    origError(...args);
  };
}

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: false } },
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <I18nProvider>
          <AppContextProvider>
            <HashRouter>
              <App />
            </HashRouter>
          </AppContextProvider>
        </I18nProvider>
      </ThemeProvider>
    </QueryClientProvider>
  </React.StrictMode>,
);
