import { type InputHTMLAttributes, forwardRef } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, className = "", id, ...rest }, ref) => {
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
          className={`rounded-md border border-vx-border bg-vx-bg-secondary px-3 py-2 text-sm text-vx-text-primary placeholder:text-vx-text-dim focus:border-vx-accent focus:outline-none focus:ring-1 focus:ring-vx-accent ${className}`}
          {...rest}
        />
      </label>
    );
  },
);
Input.displayName = "Input";
