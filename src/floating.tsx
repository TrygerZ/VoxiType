import React, { useEffect } from "react";
import ReactDOM from "react-dom/client";
import { FloatingWidget } from "./components/floating-widget/FloatingWidget";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { getSettings, onEvent, revealFloatingWidget } from "./lib/tauri";
import { applyTheme, normalizeThemePreference } from "./lib/theme";
import type { ThemeChangedEvent } from "./types/events";
import "./styles/index.css";

// Make the overlay window background transparent.
document.documentElement.classList.add("vx-transparent");
document.body.classList.add("vx-transparent");

export function FloatingApp() {
  // Tell the backend the page mounted so it can re-sync the pipeline state.
  // The window itself is revealed by the backend's page-load hook: WebKit
  // suspends requestAnimationFrame while the window is hidden, so a reveal
  // driven from here would never run on macOS or Linux.
  useEffect(() => {
    void revealFloatingWidget();
  }, []);

  // Load initial theme preference and keep the overlay synced with theme_changed events.
  useEffect(() => {
    let active = true;
    let hasEventUpdate = false;
    let cleanupTheme: () => void = () => undefined;

    const applyPreference = (rawTheme: unknown) => {
      cleanupTheme();
      cleanupTheme = applyTheme(normalizeThemePreference(rawTheme));
    };

    void getSettings()
      .then((settings) => {
        if (!active || hasEventUpdate) return;
        applyPreference(settings?.theme);
      })
      .catch(() => {});

    const unlistenPromise = onEvent<ThemeChangedEvent>("theme_changed", (payload) => {
      if (!active) return;
      hasEventUpdate = true;
      applyPreference(payload.theme);
    });

    return () => {
      active = false;
      cleanupTheme();
      cleanupTheme = () => undefined;
      unlistenPromise.then((fn) => fn()).catch(() => {});
    };
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

const rootEl = document.getElementById("root");
if (rootEl) {
  ReactDOM.createRoot(rootEl).render(
    <React.StrictMode>
      <FloatingApp />
    </React.StrictMode>,
  );
}
