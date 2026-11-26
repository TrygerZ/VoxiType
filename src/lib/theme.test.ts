import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  applyTheme,
  normalizeThemePreference,
  resolveThemeMode,
} from "./theme";

describe("theme helpers", () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    delete document.documentElement.dataset.theme;
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
    delete document.documentElement.dataset.theme;
  });

  describe("normalizeThemePreference", () => {
    it("accepts dark, light, and system", () => {
      expect(normalizeThemePreference("dark")).toBe("dark");
      expect(normalizeThemePreference("light")).toBe("light");
      expect(normalizeThemePreference("system")).toBe("system");
    });

    it("falls back to dark for unknown or non-string values", () => {
      expect(normalizeThemePreference(undefined)).toBe("dark");
      expect(normalizeThemePreference(null)).toBe("dark");
      expect(normalizeThemePreference("sepia")).toBe("dark");
      expect(normalizeThemePreference(123)).toBe("dark");
    });
  });

  describe("resolveThemeMode", () => {
    it("returns explicit dark or light directly", () => {
      expect(resolveThemeMode("dark")).toBe("dark");
      expect(resolveThemeMode("light")).toBe("light");
    });

    it("resolves system from prefers-color-scheme: light", () => {
      window.matchMedia = vi.fn().mockImplementation((query: string) => ({
        matches: query === "(prefers-color-scheme: light)",
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }));
      expect(resolveThemeMode("system")).toBe("light");

      window.matchMedia = vi.fn().mockImplementation((query: string) => ({
        matches: false,
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }));
      expect(resolveThemeMode("system")).toBe("dark");
    });

    it("falls back to dark when matchMedia is unavailable", () => {
      // @ts-expect-error simulating missing matchMedia in minimal environment
      window.matchMedia = undefined;
      expect(resolveThemeMode("system")).toBe("dark");
    });
  });

  describe("applyTheme", () => {
    it("sets data-theme on documentElement for explicit modes without media listener", () => {
      const addEventListener = vi.fn();
      window.matchMedia = vi.fn().mockImplementation(() => ({
        matches: false,
        addEventListener,
        removeEventListener: vi.fn(),
      }));

      const cleanupLight = applyTheme("light");
      expect(document.documentElement.dataset.theme).toBe("light");
      expect(addEventListener).not.toHaveBeenCalled();
      cleanupLight();

      const cleanupDark = applyTheme("dark");
      expect(document.documentElement.dataset.theme).toBe("dark");
      expect(addEventListener).not.toHaveBeenCalled();
      cleanupDark();
    });

    it("subscribes to matchMedia changes for system preference and removes listener on cleanup", () => {
      const changeHandlers: Array<(event: { matches: boolean }) => void> = [];
      const addEventListener = vi.fn(
        (_event: string, handler: (event: { matches: boolean }) => void) => {
          changeHandlers.push(handler);
        },
      );
      const removeEventListener = vi.fn();

      window.matchMedia = vi.fn().mockImplementation(() => ({
        matches: false,
        media: "(prefers-color-scheme: light)",
        addEventListener,
        removeEventListener,
      }));

      const cleanup = applyTheme("system");
      expect(document.documentElement.dataset.theme).toBe("dark");
      expect(addEventListener).toHaveBeenCalledWith("change", expect.any(Function));
      expect(changeHandlers).toHaveLength(1);

      changeHandlers[0]({ matches: true });
      expect(document.documentElement.dataset.theme).toBe("light");

      changeHandlers[0]({ matches: false });
      expect(document.documentElement.dataset.theme).toBe("dark");

      cleanup();
      expect(removeEventListener).toHaveBeenCalledWith(
        "change",
        expect.any(Function),
      );
    });
  });
});
