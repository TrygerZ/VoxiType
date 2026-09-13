import { Check, AlertCircle } from "lucide-react";

import { useToastStore, toast, type ToastType } from "../../stores/toastStore";

export { toast, type ToastType };

export function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      {toasts.map((t) => (
        <div
          key={t.id}
          className={`vx-scale-in flex items-center gap-2 rounded-lg border px-4 py-2.5 text-sm shadow-vx-md transition-all ${
            t.type === "success"
              ? "border-vx-success/30 bg-vx-success/10 text-vx-success"
              : t.type === "error"
                ? "border-vx-error/30 bg-vx-error/10 text-vx-error"
                : "border-vx-accent/30 bg-vx-accent/10 text-vx-accent"
          }`}
        >
          {t.type === "success" ? (
            <Check className="h-4 w-4" />
          ) : t.type === "error" ? (
            <AlertCircle className="h-4 w-4" />
          ) : null}
          {t.message}
        </div>
      ))}
    </div>
  );
}
