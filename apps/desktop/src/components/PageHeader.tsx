import type { ReactNode } from "react";
import { useAppContext } from "../lib/context";

/**
 * Page title row. `context` appends the current "project / environment" in
 * muted mono; `actions` sit at the right edge. No descriptions by design.
 */
export function PageHeader({ title, context = false, actions }: { title: string; context?: boolean; actions?: ReactNode }) {
  const { project, environment } = useAppContext();
  const crumbs = context && project ? [project.name, environment?.name].filter(Boolean).join(" / ") : null;
  return (
    <div className="mb-5 flex h-8 items-center justify-between gap-4">
      <div className="flex min-w-0 items-baseline gap-3">
        <h1 className="shrink-0 text-lg leading-none">{title}</h1>
        {crumbs && <span className="truncate font-mono text-xs text-muted-foreground">{crumbs}</span>}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </div>
  );
}
