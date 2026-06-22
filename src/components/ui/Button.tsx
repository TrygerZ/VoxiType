import { type ButtonHTMLAttributes, forwardRef } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variantClasses: Record<Variant, string> = {
  primary: "bg-vx-accent text-vx-bg-primary hover:bg-vx-accent-hover",
  secondary:
    "bg-vx-bg-tertiary text-vx-text-primary hover:bg-vx-bg-elevated",
  ghost:
    "bg-transparent text-vx-text-secondary hover:bg-vx-bg-tertiary hover:text-vx-text-primary",
  danger: "bg-vx-error/15 text-vx-error hover:bg-vx-error/25",
};

const sizeClasses: Record<Size, string> = {
  sm: "text-xs px-3 py-1.5 gap-1.5",
  md: "text-sm px-4 py-2 gap-2",
  lg: "text-sm px-5 py-2.5 gap-2",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "secondary", size = "md", className = "", ...rest }, ref) => {
    const base =
      "inline-flex items-center justify-center rounded-lg font-medium transition-colors duration-150 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent/40 focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-primary";
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
