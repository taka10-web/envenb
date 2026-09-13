import { createRoot } from "react-dom/client";

// Tauri's WebView does not show `window.confirm` dialogs (WKWebView needs a native
// delegate), so confirmations are rendered in-page. `confirmAsync` mounts a small
// modal into <body>, resolves with the user's choice and unmounts itself.

export interface ConfirmLabels {
  confirm: string;
  cancel: string;
}

export function confirmAsync(message: string, labels: ConfirmLabels): Promise<boolean> {
  return new Promise((resolve) => {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    const finish = (value: boolean) => {
      root.unmount();
      host.remove();
      document.removeEventListener("keydown", onKey);
      resolve(value);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") finish(false);
    };
    document.addEventListener("keydown", onKey);
    root.render(
      <div
        role="dialog"
        aria-modal="true"
        aria-label={message}
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
        onClick={() => finish(false)}
      >
        <div
          className="w-full max-w-sm rounded-lg border border-border bg-card p-5 text-card-foreground shadow-lg"
          onClick={(e) => e.stopPropagation()}
        >
          <p className="text-sm">{message}</p>
          <div className="mt-4 flex justify-end gap-2">
            <button
              type="button"
              autoFocus
              className="inline-flex h-9 items-center rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-accent"
              onClick={() => finish(false)}
            >
              {labels.cancel}
            </button>
            <button
              type="button"
              className="inline-flex h-9 items-center rounded-md bg-destructive px-4 text-sm font-medium text-white hover:bg-destructive/90"
              onClick={() => finish(true)}
            >
              {labels.confirm}
            </button>
          </div>
        </div>
      </div>,
    );
  });
}
