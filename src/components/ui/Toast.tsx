import { useEffect } from "react";
import { Check, AlertCircle } from "lucide-react";

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
  useEffect(() => {
    const timer = setTimeout(() => onDismiss(item.id), timeoutFor(item.type));
    return () => clearTimeout(timer);
  }, [item.id, item.type, onDismiss]);

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
      {item.type === "success" && <Check className="h-4 w-4" />}
      {item.type === "error" && <AlertCircle className="h-4 w-4" />}
      {item.message}
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
