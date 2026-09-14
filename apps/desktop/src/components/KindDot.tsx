import { cn } from "@envenb/ui";

export type KindTone = "teal" | "emerald" | "amber" | "sky" | "violet" | "rose" | "orange" | "slate";

const TONE: Record<KindTone, string> = {
  teal: "bg-primary",
  emerald: "bg-emerald-500",
  amber: "bg-amber-500",
  sky: "bg-sky-500",
  violet: "bg-violet-500",
  rose: "bg-rose-500",
  orange: "bg-orange-500",
  slate: "bg-slate-400 dark:bg-slate-500",
};

/** Small colored square + mono label. Replaces badges for "kind" columns. */
export function KindDot({ tone = "slate", label, className }: { tone?: KindTone; label: string; className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-1.5 font-mono text-xs", className)}>
      <span aria-hidden className={cn("h-2 w-2 shrink-0 rounded-[1px]", TONE[tone])} />
      {label}
    </span>
  );
}
