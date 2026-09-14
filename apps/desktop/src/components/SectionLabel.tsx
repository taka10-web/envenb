import type { ReactNode } from "react";
import { cn } from "@envenb/ui";

/** Section heading: small mono uppercase, optional trailing content (counts, actions). */
export function SectionLabel({ children, right, className }: { children: ReactNode; right?: ReactNode; className?: string }) {
  return (
    <div className={cn("mb-2 flex h-6 items-center justify-between gap-3", className)}>
      <h2 className="font-mono text-[11px] font-medium uppercase tracking-[0.12em] text-muted-foreground">{children}</h2>
      {right && <div className="flex items-center gap-2">{right}</div>}
    </div>
  );
}
