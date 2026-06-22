import type { ReactNode } from "react";

/** Page heading for a settings tab. */
export function SettingsHeader({
  title,
  description,
}: {
  title: string;
  description?: string;
}) {
  return (
    <div className="mb-8 flex flex-col gap-1.5">
      <h1 className="text-2xl font-semibold tracking-tight text-vx-text-primary">
        {title}
      </h1>
      {description && (
        <p className="text-sm text-vx-text-dim">{description}</p>
      )}
    </div>
  );
}

/** A grouped card of related settings rows. */
export function SettingsGroup({
  title,
  children,
}: {
  title?: string;
  children: ReactNode;
}) {
  return (
    <div className="mb-8">
      {title && (
        <h3 className="mb-1 px-1 text-[11px] font-medium uppercase tracking-[0.12em] text-vx-text-dim">
          {title}
        </h3>
      )}
      <div className="flex flex-col divide-y divide-vx-divider">{children}</div>
    </div>
  );
}

/** A single row: label/description on the left, control on the right. */
export function SettingsRow({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 py-4">
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="text-sm font-medium text-vx-text-primary">
          {label}
        </span>
        {description && (
          <span className="text-xs text-vx-text-dim">{description}</span>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
