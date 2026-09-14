import type { ReactNode } from "react";
import { cn } from "@envenb/ui";

/**
 * A row of mutually exclusive buttons (the same look as the PUBLIC/SECRET
 * toggle on the Variables tab). `tone` lets the permission matrix colour the
 * active segment by decision.
 */
export function Segmented<T extends string>({
  value,
  onChange,
  options,
  ariaLabel,
  size = "md",
  className,
}: {
  value: T | null;
  onChange: (v: T) => void;
  options: { value: T; label: ReactNode; activeClass?: string; disabled?: boolean }[];
  ariaLabel?: string;
  size?: "sm" | "md";
  className?: string;
}) {
  return (
    <div className={cn("inline-flex rounded-md border p-0.5", className)} role="group" aria-label={ariaLabel}>
      {options.map((o) => {
        const active = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            aria-pressed={active}
            disabled={o.disabled}
            onClick={() => onChange(o.value)}
            className={cn(
              "inline-flex items-center gap-1.5 rounded font-medium transition-colors disabled:opacity-50",
              size === "sm" ? "px-2.5 py-0.5 text-[11px]" : "px-3 py-1 text-xs",
              active ? (o.activeClass ?? "bg-primary text-primary-foreground") : "text-muted-foreground hover:bg-accent",
            )}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}
