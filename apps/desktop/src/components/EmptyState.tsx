import type { ReactNode } from "react";
import { cn, Maiko, useDanceFrame } from "@envenb/ui";

/** One dancer, one sentence, at most one action. */
export function EmptyState({ text, className, children }: { text: string; className?: string; children?: ReactNode }) {
  const frame = useDanceFrame();
  return (
    <div className={cn("flex flex-1 flex-col items-center justify-center gap-3 py-14 text-center", className)}>
      <Maiko figure="standing" way="red" frame={frame} size={2} />
      <p className="text-sm text-muted-foreground">{text}</p>
      {children}
    </div>
  );
}
