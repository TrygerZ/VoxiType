import { useEffect } from "react";
import { Check, AlertCircle } from "lucide-react";

import { useT } from "../../lib/i18n";
import {
  useToastStore,
  toast,
  type ToastItem,
  type ToastType,
} from "../../stores/toastStore";

export { toast, type ToastType };

const TOAST_TIMEOUT_MS = 3000;
// Errors carry a message the user must read, so they linger longer.
const TOAST_ERROR_TIMEOUT_MS = 8000;

function timeoutFor(type: ToastType): number {
  return type === "error" ? TOAST_ERROR_TIMEOUT_MS : TOAST_TIMEOUT_MS;
}

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

  const colorClasses =
    item.type === "success"
      ? "border-vx-success/30 bg-vx-success/10 text-vx-success"
      : item.type === "error"
        ? "border-vx-error/30 bg-vx-error/10 text-vx-error"
        : "border-vx-accent/30 bg-vx-accent/10 text-vx-accent";

  return (
    <div
      role={item.type === "error" ? "alert" : undefined}
      className={`vx-scale-in flex items-center gap-2 rounded-lg border px-4 py-2.5 text-sm shadow-vx-md transition-all ${colorClasses}`}
    >
      {item.type === "success" && <Check className="h-4 w-4" aria-hidden />}
      {item.type === "error" && <AlertCircle className="h-4 w-4" aria-hidden />}
      {/* The type is visual-only (color plus icon); name it for screen readers. */}
      {item.type !== "error" && (
        <span className="sr-only">{t(`toast.${item.type}_label`)}</span>
      )}
      {item.message}
      {item.action && (
        <button
          type="button"
          onClick={() => {
            item.action?.onClick();
            onDismiss(item.id);
          }}
          className="ml-1 shrink-0 rounded-md px-2 py-0.5 text-xs font-semibold underline underline-offset-2 hover:opacity-80 focus:outline-none focus-visible:ring-2 focus-visible:ring-vx-accent"
        >
          {item.action.label}
        </button>
      )}
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
      className="fixed bottom-4 right-4 z-50 flex flex-col gap-2"
    >
      {toasts.map((t) => (
        <ToastItemRow key={t.id} item={t} onDismiss={remove} />
      ))}
    </div>
  );
}
