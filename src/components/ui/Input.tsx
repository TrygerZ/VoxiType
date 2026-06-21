import { type InputHTMLAttributes, forwardRef } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, hint, className = "", id, ...rest }, ref) => {
    const inputId = id ?? rest.name;
    return (
      <label htmlFor={inputId} className="flex flex-col gap-1.5">
        {label && (
          <span className="text-xs font-medium text-vx-text-secondary">
            {label}
          </span>
        )}
        <input
          ref={ref}
          id={inputId}
          className={`rounded-lg border border-vx-border bg-vx-bg-tertiary/60 px-3.5 py-2.5 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-all duration-150 hover:border-vx-border-strong focus:border-vx-accent focus:bg-vx-bg-tertiary focus:outline-none focus:ring-2 focus:ring-vx-accent/30 ${className}`}
          {...rest}
        />
        {hint && <span className="text-xs text-vx-text-dim">{hint}</span>}
      </label>
    );
  },
);
Input.displayName = "Input";
