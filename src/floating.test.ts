import { createElement } from "react";
import { act, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import source from "./floating.tsx?raw";
import * as tauri from "./lib/tauri";
import { FloatingApp } from "./floating";

/**
 * The overlay window is created hidden and revealed by the backend's
 * page-load hook. Revealing from the page must not wait on
 * requestAnimationFrame: WebKit suspends it while the window is hidden, so
 * the widget never appears on macOS or Linux.
 */
describe("floating entry point", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    delete document.documentElement.dataset.theme;
  });

  afterEach(() => {
    delete document.documentElement.dataset.theme;
  });

  it("never gates the reveal on requestAnimationFrame", () => {
    expect(source).not.toMatch(/requestAnimationFrame\s*\(/);
  });

  it("loads initial theme from getSettings and syncs theme_changed events", async () => {
    let themeHandler: ((payload: unknown) => void) | undefined;
    vi.mocked(tauri.getSettings).mockResolvedValueOnce({ theme: "light" });
    vi.mocked(tauri.onEvent).mockImplementation((event, handler) => {
      if (event === "theme_changed") {
        themeHandler = handler as (payload: unknown) => void;
      }
      return Promise.resolve(() => undefined);
    });

    render(createElement(FloatingApp));

    await waitFor(() => {
      expect(document.documentElement.dataset.theme).toBe("light");
    });

    act(() => {
      themeHandler?.({ theme: "dark" });
    });

    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});
