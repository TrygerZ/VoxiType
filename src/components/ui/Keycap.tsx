import type { HTMLAttributes, ReactNode } from "react";

export interface KeycapProps extends HTMLAttributes<HTMLElement> {
  label?: string;
  children?: ReactNode;
}

export function Keycap({
  label,
  children,
  className = "",
  ...rest
}: KeycapProps) {
  return (
    <kbd
      className={`inline-flex items-center justify-center font-mono text-[11px] font-medium leading-[14px] bg-vx-bg-elevated border border-vx-border-default border-b-2 rounded-[4px] px-1.5 py-0.5 text-vx-text-primary shadow-vx-subtle ${className}`}
      {...rest}
    >
      {label ?? children}
    </kbd>
  );
}
