import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../lib/cn";

export const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border text-sm font-medium select-none transition-[transform,box-shadow,background-color] duration-75 disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
  {
    variants: {
      variant: {
        default:
          "border-primary bg-primary text-primary-foreground shadow-[2px_2px_0_0_var(--color-border)] hover:brightness-110 active:translate-x-px active:translate-y-px active:shadow-none",
        secondary:
          "border-border bg-secondary text-secondary-foreground shadow-[2px_2px_0_0_var(--color-border)] hover:bg-accent active:translate-x-px active:translate-y-px active:shadow-none",
        outline:
          "border-border bg-background shadow-[2px_2px_0_0_var(--color-border)] hover:bg-accent hover:text-accent-foreground active:translate-x-px active:translate-y-px active:shadow-none",
        ghost: "border-transparent hover:bg-accent hover:text-accent-foreground",
        destructive:
          "border-destructive bg-destructive text-white shadow-[2px_2px_0_0_var(--color-border)] hover:brightness-110 active:translate-x-px active:translate-y-px active:shadow-none",
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
