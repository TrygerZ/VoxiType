import type { ReactNode } from "react";

interface PanelHeaderProps {
  title: string;
  subtitle?: string;
  icon?: ReactNode;
  actions?: ReactNode;
}

/** Consistent header used across History / Dictionary / Snippets panels. */
export function PanelHeader({ title, subtitle, actions }: PanelHeaderProps) {
  return (
    <div className="flex items-end justify-between gap-4 px-10 pb-5 pt-9">
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-tight text-vx-text-primary">
          {title}
        </h1>
        {subtitle && <p className="text-sm text-vx-text-dim">{subtitle}</p>}
      </div>
      {actions && <div className="flex items-center gap-1.5">{actions}</div>}
    </div>
  );
}
