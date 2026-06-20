export function ModeLabel({ mode }: { mode: string }) {
  return (
    <span className="rounded-full bg-vx-bg-tertiary px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-vx-text-secondary">
      {mode}
    </span>
  );
}
