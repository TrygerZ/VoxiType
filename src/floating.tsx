import React from "react";
import ReactDOM from "react-dom/client";
import { FloatingWidget } from "./components/floating-widget/FloatingWidget";
import { useTauriEvents } from "./hooks/useTauriEvents";
import "./styles/index.css";

// Make the overlay window background transparent.
document.documentElement.classList.add("vx-transparent");
document.body.classList.add("vx-transparent");

function FloatingApp() {
  // Reuse the same event subscriptions as the main window so the widget
  // reflects recording state, audio level, timer, and results live.
  useTauriEvents();
  return (
    <div className="flex h-screen w-screen items-center justify-center bg-transparent overflow-hidden p-6">
      <FloatingWidget alwaysRender />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <FloatingApp />
  </React.StrictMode>,
);
