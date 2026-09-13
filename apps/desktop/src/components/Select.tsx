import * as React from "react";
import { cn } from "@envfish/ui";

/** Native <select> styled like the shared Input primitive. */
export function Select({ className, ...props }: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      className={cn(
        "flex h-9 w-full border-2 border-input bg-background px-3 py-1 text-sm  transition-colors focus-visible:outline-none focus-visible:border-primary focus-visible:ring-0 disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    />
  );
}
