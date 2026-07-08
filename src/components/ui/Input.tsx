import { type InputHTMLAttributes, forwardRef, useId, useState } from "react";
import { Eye, EyeOff } from "lucide-react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
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
            className={`w-full rounded-lg bg-vx-bg-tertiary px-3.5 py-2.5 text-sm text-vx-text-primary placeholder:text-vx-text-dim transition-shadow duration-150 focus:outline-none focus:ring-2 focus:ring-vx-accent/40 ${
              showPasswordToggle ? "pr-10" : ""
            } ${className}`}
            {...rest}
          />
          {isPassword && showPasswordToggle && (
            <button
              type="button"
              onClick={() => setVisible((v) => !v)}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded text-vx-text-dim hover:text-vx-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent/40 transition-colors"
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
        {hint && <span className="text-xs text-vx-text-dim">{hint}</span>}
      </label>
    );
  },
);
Input.displayName = "Input";
