import type { ThemePreference } from "../types/app";

const LIGHT_COLOR_SCHEME_QUERY = "(prefers-color-scheme: light)";

function getLightMediaQuery(): MediaQueryList | null {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return null;
  }
  return window.matchMedia(LIGHT_COLOR_SCHEME_QUERY);
}

function resolveModeFromMedia(
  pref: ThemePreference,
  media: MediaQueryList | null,
): "dark" | "light" {
  if (pref === "light") return "light";
  if (pref === "dark") return "dark";
  return media?.matches ? "light" : "dark";
}

function setDocumentTheme(mode: "dark" | "light"): void {
  if (typeof document !== "undefined") {
    document.documentElement.dataset.theme = mode;
  }
}

export function normalizeThemePreference(value: unknown): ThemePreference {
  if (value === "dark" || value === "light" || value === "system") {
    return value;
  }
  return "dark";
}

export function resolveThemeMode(pref: ThemePreference): "dark" | "light" {
  if (pref !== "system") {
    return pref;
  }
  return resolveModeFromMedia(pref, getLightMediaQuery());
}

export function applyTheme(pref: ThemePreference): () => void {
  const media = pref === "system" ? getLightMediaQuery() : null;
  setDocumentTheme(resolveModeFromMedia(pref, media));

  if (pref !== "system" || !media || typeof media.addEventListener !== "function") {
    return () => undefined;
  }

  const handleChange = (event: MediaQueryListEvent) => {
    setDocumentTheme(event.matches ? "light" : "dark");
  };

  media.addEventListener("change", handleChange);
  return () => {
    if (typeof media.removeEventListener === "function") {
      media.removeEventListener("change", handleChange);
    }
  };
}
