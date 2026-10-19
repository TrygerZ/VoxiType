import React, { useEffect } from "react";
import ReactDOM from "react-dom/client";
import { FloatingWidget } from "./components/floating-widget/FloatingWidget";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { revealFloatingWidget } from "./lib/tauri";
import "./styles/index.css";

// Make the overlay window background transparent.
document.documentElement.classList.add("vx-transparent");
document.body.classList.add("vx-transparent");

function FloatingApp() {
  // Tell the backend the page mounted so it can re-sync the pipeline state.
  // The window itself is revealed by the backend's page-load hook: WebKit
  // suspends requestAnimationFrame while the window is hidden, so a reveal
  // driven from here would never run on macOS or Linux.
  useEffect(() => {
    void revealFloatingWidget();
  }, []);

  // Reuse the same event subscriptions as the main window so the widget
  // reflects recording state, audio level, timer, and results live,
  // without triggering history or stats reloads that the overlay does not use.
  useTauriEvents({ reloadData: false });
  return (
    // Transparent, click-through wrapper; only the pill itself is interactive.
    <div className="pointer-events-none flex h-screen w-screen items-center justify-center bg-transparent overflow-hidden p-2">
      <FloatingWidget alwaysRender />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <FloatingApp />
  </React.StrictMode>,
);
