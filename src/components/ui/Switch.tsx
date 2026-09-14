interface SwitchProps {
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
      className={`inline-flex items-center gap-3 text-left rounded-lg focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent/40 focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-primary ${
        disabled ? "opacity-40 cursor-not-allowed" : ""
      }`}
    >
      <span
        className={`relative h-5 w-9 shrink-0 rounded-full transition-colors duration-200 ease-in-out ${
          checked ? "bg-vx-accent" : "bg-vx-border-strong"
        }`}
      >
        <span
          className={`absolute top-0.5 h-4 w-4 rounded-full bg-white shadow-vx-sm transition-transform duration-200 ease-in-out ${
            checked ? "translate-x-4" : "translate-x-0.5"
          }`}
        />
      </span>
      {label && (
        <span className="text-sm text-vx-text-secondary">{label}</span>
      )}
    </button>
  );
}
