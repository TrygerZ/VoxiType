import type { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  className?: string;
}

export function Card({ children, className = "" }: CardProps) {
  return (
    <div
      className={`rounded-xl border border-vx-border bg-vx-bg-secondary/80 p-4 shadow-vx-sm ${className}`}
    >
      {children}
    </div>
  );
}
