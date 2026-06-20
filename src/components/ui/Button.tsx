import { type ButtonHTMLAttributes, forwardRef } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variantClasses: Record<Variant, string> = {
  primary:
    "bg-vx-accent text-white hover:bg-vx-accent-hover border-transparent",
  secondary:
    "bg-vx-bg-tertiary text-vx-text-primary hover:bg-vx-border border-vx-border",
  ghost:
    "bg-transparent text-vx-text-secondary hover:bg-vx-bg-tertiary border-transparent",
  danger:
    "bg-vx-error text-white hover:opacity-90 border-transparent",
};

const sizeClasses: Record<Size, string> = {
  sm: "text-xs px-2.5 py-1.5",
  md: "text-sm px-3 py-2",
  lg: "text-base px-4 py-2.5",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "secondary", size = "md", className = "", ...rest }, ref) => {
    const base =
      "inline-flex items-center justify-center gap-2 rounded-md border font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-vx-accent/40";
    return (
      <button
        ref={ref}
        className={`${base} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
        {...rest}
      />
    );
  },
);
Button.displayName = "Button";
