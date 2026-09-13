import * as React from "react";
import { cn } from "../lib/cn";

export function Input({ className, type, ...props }: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      type={type}
      className={cn(
        "flex h-9 w-full border-2 border-input bg-background px-3 py-1 text-sm shadow-[inset_2px_2px_0_0_color-mix(in_oklch,var(--color-foreground)_12%,transparent)] transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:border-primary focus-visible:ring-0 disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    />
  );
}
