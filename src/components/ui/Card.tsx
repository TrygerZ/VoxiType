import type { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  className?: string;
}

export function Card({ children, className = "" }: CardProps) {
  return (
    <div
      className={`rounded-2xl bg-vx-bg-secondary p-6 shadow-vx-md ${className}`}
    >
      {children}
    </div>
  );
}
