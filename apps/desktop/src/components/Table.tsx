import type { HTMLAttributes, TdHTMLAttributes, ThHTMLAttributes } from "react";
import { cn } from "@envfish/ui";

// Dense list/table building blocks: 40px rows, hairline dividers, hover tint,
// and row actions that appear on hover but stay reachable by keyboard.

export function Table({ className, ...props }: HTMLAttributes<HTMLTableElement>) {
  return (
    <div className="-mx-2 overflow-x-auto">
      <table className={cn("w-full text-sm", className)} {...props} />
    </div>
  );
}

export function Th({ className, ...props }: ThHTMLAttributes<HTMLTableCellElement>) {
  return (
    <th
      className={cn("h-8 whitespace-nowrap border-b border-border/60 px-2 text-left font-mono text-[11px] font-medium uppercase tracking-[0.12em] text-muted-foreground", className)}
      {...props}
    />
  );
}

export function Tr({ className, ...props }: HTMLAttributes<HTMLTableRowElement>) {
  return <tr className={cn("group h-10 border-b border-border/60 transition-colors last:border-0 hover:bg-accent/40", className)} {...props} />;
}

export function Td({ className, ...props }: TdHTMLAttributes<HTMLTableCellElement>) {
  return <td className={cn("px-2 py-0 align-middle", className)} {...props} />;
}

/** Right-aligned ghost actions; hidden until hover, visible while focused. */
export function RowActions({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex items-center justify-end gap-0.5 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100", className)} {...props} />;
}

/** Non-table list row with the same rhythm. */
export function ListRow({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("group flex min-h-10 items-center gap-3 border-b border-border/60 px-2 transition-colors last:border-0 hover:bg-accent/40", className)} {...props} />;
}
