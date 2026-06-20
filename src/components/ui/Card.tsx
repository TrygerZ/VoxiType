import type { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  className?: string;
}

export function Card({ children, className = "" }: CardProps) {
  return (
    <div
      className={`rounded-lg border border-vx-border bg-vx-bg-secondary p-4 ${className}`}
    >
      {children}
    </div>
  );
}
