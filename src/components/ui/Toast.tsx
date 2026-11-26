import { type ComponentType, useEffect } from "react";
import { AlertTriangle, Check, Info, X } from "lucide-react";

import { useT } from "../../lib/i18n";
import {
  useToastStore,
  toast,
  type ToastItem,
  type ToastType,
} from "../../stores/toastStore";
import { IconButton } from "./IconButton";

export { toast, type ToastType };

const TOAST_TIMEOUT_MS = 3000;
// Errors carry a message the user must read, so they linger longer.
const TOAST_ERROR_TIMEOUT_MS = 8000;

function timeoutFor(type: ToastType): number {
  return type === "error" ? TOAST_ERROR_TIMEOUT_MS : TOAST_TIMEOUT_MS;
}

interface ToastVisual {
  wellClass: string;
  Icon: ComponentType<{
    className?: string;
    strokeWidth?: number;
    "aria-hidden"?: boolean;
  }>;
}

const toastVisuals: Record<ToastType, ToastVisual> = {
  success: {
    wellClass: "bg-vx-semantic-success-soft text-vx-semantic-success",
    Icon: Check,
  },
  error: {
    wellClass: "bg-vx-semantic-error-soft text-vx-semantic-error",
    Icon: AlertTriangle,
  },
  info: {
    wellClass: "bg-vx-bg-subtle text-vx-text-primary",
    Icon: Info,
  },
};

function ToastItemRow({
  item,
  onDismiss,
}: {
  item: ToastItem;
  onDismiss: (id: number) => void;
}) {
  const t = useT();

  // An actionable toast stays until the user acts or dismisses it; auto-hiding
  // the recovery path is worse than a lingering message.
  useEffect(() => {
    if (item.action) return;
    const timer = setTimeout(() => onDismiss(item.id), timeoutFor(item.type));
    return () => clearTimeout(timer);
  }, [item.id, item.type, item.action, onDismiss]);

  const { wellClass, Icon } = toastVisuals[item.type];

  return (
    <div
      data-testid="toast-card"
      role={item.type === "error" ? "alert" : undefined}
      className="vx-toast-in flex items-start gap-3 min-w-[300px] max-w-[392px] rounded-[10px] border border-vx-border-default bg-vx-bg-elevated px-3.5 py-3 shadow-vx-dock"
    >
      <div
        className={`h-7 w-7 shrink-0 rounded-[7px] border border-vx-border-subtle flex items-center justify-center ${wellClass}`}
      >
        <Icon className="h-3.5 w-3.5" strokeWidth={1.75} aria-hidden />
      </div>

      <div className="min-w-0 flex-1 space-y-0.5 pt-0.5">
        {/* The type is visual-only (icon well); name it for screen readers. */}
        {item.type !== "error" && (
          <span className="sr-only">{t(`toast.${item.type}_label`)}</span>
        )}
        <p className="text-[13px] font-medium leading-[18px] text-vx-text-primary">
          {item.message}
        </p>
        {item.description && (
          <p className="text-xs leading-4 text-vx-text-secondary">
            {item.description}
          </p>
        )}
      </div>

      <div className="flex items-center gap-2 shrink-0 pt-0.5">
        {item.action && (
          <button
            type="button"
            onClick={() => {
              item.action?.onClick();
              onDismiss(item.id);
            }}
            className="rounded-[6px] border border-vx-border-default bg-vx-bg-subtle px-2 py-1 text-xs font-medium text-vx-text-primary hover:bg-vx-bg-surface focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent-ember"
          >
            {item.action.label}
          </button>
        )}
        <IconButton
          icon={<X className="h-3.5 w-3.5" strokeWidth={1.75} />}
          label={t("toast.dismiss")}
          tone="ghost"
          size="sm"
          className="h-6 w-6"
          onClick={() => onDismiss(item.id)}
        />
      </div>
    </div>
  );
}

export function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);
  const remove = useToastStore((s) => s.remove);

  if (toasts.length === 0) return null;

  return (
    <div
      aria-live="polite"
      className="fixed top-4 right-4 z-50 flex flex-col gap-2"
    >
      {toasts.map((item) => (
        <ToastItemRow key={item.id} item={item} onDismiss={remove} />
      ))}
    </div>
  );
}
