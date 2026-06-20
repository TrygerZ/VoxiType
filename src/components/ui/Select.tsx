import { type SelectHTMLAttributes, forwardRef } from "react";

interface Option {
  value: string;
  label: string;
}

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  options: Option[];
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  ({ label, options, className = "", id, ...rest }, ref) => {
    const selectId = id ?? rest.name;
    return (
      <label htmlFor={selectId} className="flex flex-col gap-1.5">
        {label && (
          <span className="text-xs font-medium text-vx-text-secondary">
            {label}
          </span>
        )}
        <select
          ref={ref}
          id={selectId}
          className={`rounded-md border border-vx-border bg-vx-bg-secondary px-3 py-2 text-sm text-vx-text-primary focus:border-vx-accent focus:outline-none focus:ring-1 focus:ring-vx-accent ${className}`}
          {...rest}
        >
          {options.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      </label>
    );
  },
);
Select.displayName = "Select";
