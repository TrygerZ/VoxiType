import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSettingsStore } from "./settingsStore";
import * as tauri from "../lib/tauri";

describe("settingsStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({
      settings: {},
      loaded: false,
      error: null,
    });
  });

  it("sets error to null and populates settings on successful load", async () => {
    vi.mocked(tauri.getSettings).mockResolvedValueOnce({
      onboarding_completed: true,
      language: "en",
    });

    await useSettingsStore.getState().load();

    const state = useSettingsStore.getState();
    expect(state.loaded).toBe(true);
    expect(state.error).toBeNull();
    expect(state.settings).toEqual({
      onboarding_completed: true,
      language: "en",
    });
  });

  it("sets error and keeps empty settings on load failure", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.mocked(tauri.getSettings).mockRejectedValueOnce(
      new Error("database locked"),
    );

    await useSettingsStore.getState().load();

    const state = useSettingsStore.getState();
    expect(state.loaded).toBe(true);
    expect(state.error).toBe("database locked");
    expect(state.settings).toEqual({});
    expect(consoleSpy).toHaveBeenCalled();
    consoleSpy.mockRestore();
  });

  it("resets error on subsequent successful retry", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.mocked(tauri.getSettings).mockRejectedValueOnce(
      new Error("temporary error"),
    );

    await useSettingsStore.getState().load();
    expect(useSettingsStore.getState().error).toBe("temporary error");

    vi.mocked(tauri.getSettings).mockResolvedValueOnce({
      onboarding_completed: true,
    });

    await useSettingsStore.getState().load();
    const state = useSettingsStore.getState();
    expect(state.error).toBeNull();
    expect(state.settings).toEqual({ onboarding_completed: true });
    consoleSpy.mockRestore();
  });

  it("drops stale responses when overlapping load calls resolve out of order", async () => {
    let resolveFirst!: (value: Record<string, unknown>) => void;
    const firstPromise = new Promise<Record<string, unknown>>((resolve) => {
      resolveFirst = resolve;
    });

    vi.mocked(tauri.getSettings)
      .mockImplementationOnce(() => firstPromise)
      .mockResolvedValueOnce({ active_mode: "email" });

    const load1 = useSettingsStore.getState().load();
    const load2 = useSettingsStore.getState().load();

    await load2;
    expect(useSettingsStore.getState().settings).toEqual({ active_mode: "email" });

    resolveFirst({ active_mode: "dictation" });
    await load1;

    expect(useSettingsStore.getState().settings).toEqual({ active_mode: "email" });
  });
});
