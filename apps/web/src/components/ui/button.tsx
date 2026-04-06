import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

const buttonVariants = cva(
  "relative inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-all duration-300 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--ring-strong))]/55 disabled:pointer-events-none disabled:opacity-50 active:scale-[0.99]",
  {
    variants: {
      variant: {
        default:
          "border border-primary/35 bg-gradient-to-br from-primary/95 via-primary to-primary/80 text-primary-foreground shadow-glow hover:brightness-110",
        secondary:
          "border border-[hsl(var(--border-strong))]/40 bg-surface-2/90 text-secondary-foreground shadow-soft hover:border-primary/35 hover:bg-surface-3/90",
        ghost: "text-muted-foreground hover:bg-surface-2/70 hover:text-foreground",
      },
      size: {
        default: "h-10 px-4 py-2",
        sm: "h-9 rounded-md px-3",
        lg: "h-11 rounded-md px-8",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => {
    return <button className={cn(buttonVariants({ variant, size, className }))} ref={ref} {...props} />;
  }
);
Button.displayName = "Button";

export { Button, buttonVariants };
