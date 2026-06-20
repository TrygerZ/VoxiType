import { Mic } from "lucide-react";
import { useAppStore } from "../../stores/appStore";
import { FloatingWidget } from "../floating-widget/FloatingWidget";
import { startRecording } from "../../lib/tauri";
import { t } from "../../lib/i18n";

export function HomeView() {
  const state = useAppStore((s) => s.state);

  return (
    <div className="flex h-full flex-col items-center justify-center gap-6 p-8">
      <FloatingWidget />
      {state === "idle" && (
        <>
          <button
            type="button"
            onClick={() => void startRecording()}
            className="group rounded-full bg-vx-accent/10 p-10 transition-colors hover:bg-vx-accent/20"
          >
            <Mic className="h-12 w-12 text-vx-accent transition-transform group-hover:scale-110" />
          </button>
          <p className="text-sm text-vx-text-dim">{t("idle.hint")}</p>
        </>
      )}
    </div>
  );
}
