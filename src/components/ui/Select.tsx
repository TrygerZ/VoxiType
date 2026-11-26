import { type SelectHTMLAttributes, forwardRef, useId } from "react";
import { ChevronDown } from "lucide-react";

interface Option {
  value: string;
  label: string;
}

export interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  options: Option[];
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  ({ label, options, className = "", id, ...rest }, ref) => {
    const defaultId = useId();
    const selectId = id ?? defaultId;
    return (
      <label htmlFor={selectId} className="flex flex-col gap-1.5">
        {label && (
          <span className="text-xs font-medium text-vx-text-secondary">
            {label}
          </span>
        )}
        <div className="relative has-[:disabled]:opacity-50">
          <select
            ref={ref}
            id={selectId}
            className={`w-full appearance-none rounded-[8px] bg-vx-bg-subtle border border-vx-border-default px-3 py-2 pr-9 text-[13px] leading-[18px] text-vx-text-primary transition-[border-color,box-shadow] duration-[120ms] ease-[var(--ease-vx-out)] focus:outline-none focus:border-vx-accent-ember focus:ring-1 focus:ring-vx-accent-ember disabled:cursor-not-allowed ${className}`}
            {...rest}
          >
            {options.map((o) => (
              <option
                key={o.value}
                value={o.value}
                className="bg-vx-bg-surface text-vx-text-primary"
              >
                {o.label}
              </option>
            ))}
          </select>
          <ChevronDown className="pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-vx-text-muted" />
        </div>
      </label>
    );
  },
);
Select.displayName = "Select";
