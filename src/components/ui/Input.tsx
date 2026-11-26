import { type InputHTMLAttributes, forwardRef, useId, useState } from "react";
import { Eye, EyeOff } from "lucide-react";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: string;
  showPasswordToggle?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, hint, showPasswordToggle, className = "", id, type, ...rest }, ref) => {
    const defaultId = useId();
    const inputId = id ?? defaultId;
    const [visible, setVisible] = useState(false);
    const isPassword = type === "password";
    const inputType = isPassword && visible ? "text" : type;

    return (
      <label htmlFor={inputId} className="flex flex-col gap-1.5">
        {label && (
          <span className="text-xs font-medium text-vx-text-secondary">
            {label}
          </span>
        )}
        <div className="relative">
          <input
            ref={ref}
            id={inputId}
            type={inputType}
            className={`w-full bg-vx-bg-subtle border border-vx-border-default rounded-[8px] px-3 py-2 text-[13px] leading-[18px] text-vx-text-primary placeholder:text-vx-text-muted transition-[border-color,box-shadow] duration-[120ms] ease-[var(--ease-vx-out)] focus:outline-none focus:border-vx-accent-ember focus:ring-1 focus:ring-vx-accent-ember ${
              showPasswordToggle ? "pr-10" : ""
            } ${className}`}
            {...rest}
          />
          {isPassword && showPasswordToggle && (
            <button
              type="button"
              onClick={() => setVisible((v) => !v)}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded-[5px] text-vx-text-muted hover:text-vx-text-primary focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember focus-visible:ring-offset-1 focus-visible:ring-offset-vx-bg-canvas transition-colors"
              aria-label={
                visible
                  ? label
                    ? `Hide ${label}`
                    : "Hide password"
                  : label
                    ? `Show ${label}`
                    : "Show password"
              }
            >
              {visible ? (
                <EyeOff className="h-4 w-4" />
              ) : (
                <Eye className="h-4 w-4" />
              )}
            </button>
          )}
        </div>
        {hint && <span className="text-xs text-vx-text-muted">{hint}</span>}
      </label>
    );
  },
);
Input.displayName = "Input";
