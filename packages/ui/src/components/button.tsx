import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../lib/cn";

export const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap border-2 text-sm font-medium select-none transition-[transform,box-shadow] duration-75 disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
  {
    variants: {
      variant: {
        default:
          "border-foreground bg-primary text-primary-foreground shadow-[3px_3px_0_0_var(--color-foreground)] hover:brightness-110 active:translate-x-[3px] active:translate-y-[3px] active:shadow-none",
        secondary:
          "border-foreground bg-secondary text-secondary-foreground shadow-[3px_3px_0_0_var(--color-foreground)] hover:bg-accent active:translate-x-[3px] active:translate-y-[3px] active:shadow-none",
        outline:
          "border-border bg-background shadow-[3px_3px_0_0_var(--color-border)] hover:bg-accent hover:text-accent-foreground active:translate-x-[3px] active:translate-y-[3px] active:shadow-none",
        ghost: "border-transparent hover:border-border hover:bg-accent hover:text-accent-foreground",
        destructive:
          "border-foreground bg-destructive text-white shadow-[3px_3px_0_0_var(--color-foreground)] hover:brightness-110 active:translate-x-[3px] active:translate-y-[3px] active:shadow-none",
        link: "border-transparent text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 px-3 text-xs",
        lg: "h-10 px-8",
        icon: "h-9 w-9",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export function Button({ className, variant, size, asChild = false, ...props }: ButtonProps) {
  const Comp = asChild ? Slot : "button";
  return <Comp className={cn(buttonVariants({ variant, size, className }))} {...props} />;
}
