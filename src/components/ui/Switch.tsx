export interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  ariaLabel?: string;
  disabled?: boolean;
  "data-testid"?: string;
}

export function Switch({
  checked,
  onChange,
  label,
  ariaLabel,
  disabled = false,
  "data-testid": testId,
}: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel || label || undefined}
      disabled={disabled}
      data-testid={testId}
      onClick={() => {
        if (!disabled) {
          onChange(!checked);
        }
      }}
      className={`inline-flex items-center gap-3 text-left rounded-[8px] focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-canvas ${
        disabled ? "opacity-40 cursor-not-allowed" : ""
      }`}
    >
      <span
        className={`relative h-5 w-9 shrink-0 rounded-full border transition-colors duration-[180ms] ease-[var(--ease-vx-out)] ${
          checked
            ? "bg-vx-bg-inverse border-vx-bg-inverse"
            : "bg-vx-bg-elevated border-vx-border-strong"
        }`}
      >
        <span
          className={`absolute top-1/2 -translate-y-1/2 h-3.5 w-3.5 rounded-full transition-transform duration-[180ms] ease-[var(--ease-vx-out)] ${
            checked
              ? "translate-x-[17px] bg-vx-text-inverse"
              : "translate-x-[3px] bg-vx-text-primary"
          }`}
        />
      </span>
      {label && (
        <span className="text-sm text-vx-text-secondary">{label}</span>
      )}
    </button>
  );
}
