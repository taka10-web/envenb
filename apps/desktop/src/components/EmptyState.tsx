import type { ReactNode } from "react";
import { cn, Goldfish } from "@envfish/ui";

/** One fish, one sentence, at most one action. */
export function EmptyState({ text, className, children }: { text: string; className?: string; children?: ReactNode }) {
  return (
    <div className={cn("flex flex-col items-center gap-3 py-14 text-center", className)}>
      <Goldfish variant="nishiki" size={5} />
      <p className="text-sm text-muted-foreground">{text}</p>
      {children}
    </div>
  );
}
