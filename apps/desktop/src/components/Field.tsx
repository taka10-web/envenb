import type { ReactNode } from "react";
import { Label } from "@envfish/ui";

/** Stacked label + control used inside sheet forms. */
export function Field({ label, hint, htmlFor, children }: { label: ReactNode; hint?: string; htmlFor: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={htmlFor} className="text-xs">
        {label}
        {hint && <span className="ml-1 font-normal text-muted-foreground">({hint})</span>}
      </Label>
      {children}
    </div>
  );
}
