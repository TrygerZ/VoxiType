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
  // Ask the backend to reveal the overlay window now that its transparent
  // content has mounted; this avoids a white-square flash over the
  // animation layer while the page is still loading (esp. in tauri dev).
  useEffect(() => {
    void revealFloatingWidget();
  }, []);

  // Reuse the same event subscriptions as the main window so the widget
  // reflects recording state, audio level, timer, and results live.
  useTauriEvents();
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
