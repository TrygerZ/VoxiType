import type { ButtonHTMLAttributes, ReactNode } from "react";
import { forwardRef } from "react";

export interface IconButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: ReactNode;
  label: string;
  tone?: "ghost" | "danger";
  size?: "xs" | "sm" | "md";
}

const toneClasses: Record<NonNullable<IconButtonProps["tone"]>, string> = {
  ghost:
    "bg-transparent text-vx-text-muted hover:bg-vx-bg-subtle hover:text-vx-text-primary",
  danger:
    "bg-vx-semantic-error-soft border border-vx-border-subtle text-vx-semantic-error hover:opacity-90",
};

const sizeClasses: Record<NonNullable<IconButtonProps["size"]>, string> = {
  xs: "h-6 w-6 rounded-[5px]",
  sm: "h-7 w-7 rounded-[6px]",
  md: "h-[30px] w-[30px] rounded-[6px]",
};

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  (
    {
      icon,
      label,
      tone = "ghost",
      size = "sm",
      type,
      title,
      className = "",
      ...rest
    },
    ref,
  ) => {
    const base =
      "inline-flex items-center justify-center shrink-0 transition-[background-color,border-color,color,opacity] duration-[120ms] ease-[var(--ease-vx-out)] disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-canvas";
    return (
      <button
        ref={ref}
        type={type ?? "button"}
        title={title ?? label}
        className={`${base} ${toneClasses[tone]} ${sizeClasses[size]} ${className}`}
        {...rest}
        aria-label={label}
      >
        {icon}
      </button>
    );
  },
);
IconButton.displayName = "IconButton";
