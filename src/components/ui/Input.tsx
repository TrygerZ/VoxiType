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
          className={`rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-shadow duration-150 focus:outline-none focus:ring-2 focus:ring-vx-accent/40 ${className}`}
          {...rest}
        />
        {hint && <span className="text-xs text-vx-text-dim">{hint}</span>}
      </label>
    );
  },
);
Input.displayName = "Input";
