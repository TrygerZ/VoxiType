import { type ButtonHTMLAttributes, forwardRef } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variantClasses: Record<Variant, string> = {
  primary:
    "bg-vx-accent text-white border-transparent shadow-vx-sm hover:bg-vx-accent-hover active:scale-[0.98]",
  secondary:
    "bg-vx-bg-tertiary text-vx-text-primary border-vx-border hover:bg-vx-bg-elevated hover:border-vx-border-strong active:scale-[0.98]",
  ghost:
    "bg-transparent text-vx-text-secondary border-transparent hover:bg-vx-bg-tertiary hover:text-vx-text-primary",
  danger:
    "bg-vx-error text-white border-transparent shadow-vx-sm hover:opacity-90 active:scale-[0.98]",
};

const sizeClasses: Record<Size, string> = {
  sm: "text-xs px-2.5 py-1.5 gap-1.5",
  md: "text-sm px-3.5 py-2 gap-2",
  lg: "text-base px-5 py-2.5 gap-2",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "secondary", size = "md", className = "", ...rest }, ref) => {
    const base =
      "inline-flex items-center justify-center rounded-lg border font-medium transition-all duration-150 disabled:opacity-50 disabled:cursor-not-allowed disabled:active:scale-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent/50 focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-primary";
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
