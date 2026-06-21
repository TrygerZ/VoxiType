export function ModeLabel({ mode }: { mode: string }) {
  return (
    <span className="rounded-full border border-vx-accent/30 bg-vx-accent-soft px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-vx-accent">
      {mode}
    </span>
  );
}
