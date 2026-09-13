import type { ReactNode } from "react";
import { cn, Goldfish, type GoldfishVariant } from "@envfish/ui";

export function EmptyState({
  text,
  variant = "nishiki",
  className,
  children,
}: {
  text: string;
  variant?: GoldfishVariant;
  className?: string;
  children?: ReactNode;
}) {
  return (
    <div className={cn("flex flex-col items-center gap-2 py-12 text-center text-muted-foreground", className)}>
      <Goldfish variant={variant} size={6} />
      <p className="text-sm">{text}</p>
      {children}
    </div>
  );
}
