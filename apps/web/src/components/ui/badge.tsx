import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-full px-2.5 py-1 text-xs font-medium ring-1 ring-inset backdrop-blur-sm transition-colors",
  {
    variants: {
      variant: {
        default: "border border-primary/35 bg-primary/18 text-primary-foreground ring-primary/28",
        success: "border border-success/40 bg-success/20 text-success-foreground ring-success/30",
        muted: "border border-border/80 bg-surface-2/95 text-muted-foreground ring-border/70",
        danger: "border border-red-500/45 bg-red-500/20 text-red-200 ring-red-500/30",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
);

export interface BadgeProps extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };
