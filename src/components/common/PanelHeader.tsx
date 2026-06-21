import type { ReactNode } from "react";

interface PanelHeaderProps {
  title: string;
  subtitle?: string;
  icon?: ReactNode;
  actions?: ReactNode;
}

/** Consistent header used across History / Dictionary / Snippets panels. */
export function PanelHeader({
  title,
  subtitle,
  icon,
  actions,
}: PanelHeaderProps) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-vx-border px-5 py-4">
      <div className="flex items-center gap-3">
        {icon && (
          <span className="flex h-9 w-9 items-center justify-center rounded-xl bg-vx-accent-soft text-vx-accent">
            {icon}
          </span>
        )}
        <div className="flex flex-col">
          <h2 className="text-base font-semibold tracking-tight">{title}</h2>
          {subtitle && (
            <p className="text-xs text-vx-text-dim">{subtitle}</p>
          )}
        </div>
      </div>
      {actions && <div className="flex items-center gap-1.5">{actions}</div>}
    </div>
  );
}
