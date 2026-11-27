import { type ButtonHTMLAttributes, forwardRef } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger" | "ember";
type Size = "sm" | "md" | "lg";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variantClasses: Record<Variant, string> = {
  primary: "bg-vx-bg-inverse text-vx-text-inverse hover:opacity-90",
  secondary:
    "bg-vx-bg-subtle border border-vx-border-default text-vx-text-primary hover:bg-vx-bg-elevated",
  ghost:
    "bg-transparent text-vx-text-secondary hover:bg-vx-bg-subtle hover:text-vx-text-primary",
  danger: "bg-vx-semantic-error-soft text-vx-semantic-error hover:opacity-90",
  ember: "bg-vx-accent-ember text-vx-stone-0 hover:opacity-90",
};

const sizeClasses: Record<Size, string> = {
  sm: "h-[30px] px-3 text-[13px] gap-1.5",
  md: "h-9 px-4 text-[13px] gap-2",
  lg: "h-10 px-5 text-sm gap-2",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "secondary", size = "md", className = "", ...rest }, ref) => {
    const base =
      "inline-flex items-center justify-center rounded-[8px] font-medium transition-[background-color,border-color,color,opacity,transform] duration-[120ms] ease-[var(--ease-vx-out)] active:scale-[0.97] disabled:opacity-50 disabled:cursor-not-allowed disabled:active:scale-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-canvas";
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
