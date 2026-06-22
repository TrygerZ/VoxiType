import { type SelectHTMLAttributes, forwardRef } from "react";
import { ChevronDown } from "lucide-react";

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
        <div className="relative">
          <select
            ref={ref}
            id={selectId}
            className={`w-full appearance-none rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 pr-9 text-sm text-vx-text-primary transition-shadow duration-150 focus:outline-none focus:ring-2 focus:ring-vx-accent/40 ${className}`}
            {...rest}
          >
            {options.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
          <ChevronDown className="pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-vx-text-dim" />
        </div>
      </label>
    );
  },
);
Select.displayName = "Select";
