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

  it("reloads settings after failed optimistic update", async () => {
    useSettingsStore.setState({
      settings: { language: "en" },
      loaded: true,
      error: null,
    });

    vi.mocked(tauri.updateSetting).mockRejectedValueOnce(new Error("disk full"));
    vi.mocked(tauri.getSettings).mockResolvedValueOnce({ language: "en" });

    await expect(
      useSettingsStore.getState().update("language", "id"),
    ).rejects.toThrow("disk full");

    expect(tauri.getSettings).toHaveBeenCalled();
    expect(useSettingsStore.getState().settings.language).toBe("en");
  });

  it("reloads settings after failed whisper paths update", async () => {
    useSettingsStore.setState({
      settings: { whisper_cpp_binary_path: "/old/bin" },
      loaded: true,
      error: null,
    });

    vi.mocked(tauri.setWhisperCppPaths).mockRejectedValueOnce(
      new Error("invalid path"),
    );
    vi.mocked(tauri.getSettings).mockResolvedValueOnce({
      whisper_cpp_binary_path: "/old/bin",
    });

    await expect(
      useSettingsStore.getState().updateWhisperPaths("/new/bin", null),
    ).rejects.toThrow("invalid path");

    expect(tauri.getSettings).toHaveBeenCalled();
    expect(useSettingsStore.getState().settings.whisper_cpp_binary_path).toBe(
      "/old/bin",
    );
  });

  it("masks groq_api_key in store and sets groq_api_key_set indicator", async () => {
    vi.mocked(tauri.updateSetting).mockResolvedValueOnce();

    await useSettingsStore.getState().update("groq_api_key", "gsk_secret_123");

    const state = useSettingsStore.getState();
    expect(state.settings.groq_api_key).toBe("");
    expect(state.settings.groq_api_key_set).toBe(true);
    expect(tauri.updateSetting).toHaveBeenCalledWith(
      "groq_api_key",
      "gsk_secret_123",
    );
  });

  it("updates groq_api_key_set to false when key is cleared", async () => {
    useSettingsStore.setState({
      settings: { groq_api_key: "", groq_api_key_set: true },
      loaded: true,
      error: null,
    });
    vi.mocked(tauri.updateSetting).mockResolvedValueOnce();

    await useSettingsStore.getState().update("groq_api_key", "");

    const state = useSettingsStore.getState();
    expect(state.settings.groq_api_key).toBe("");
    expect(state.settings.groq_api_key_set).toBe(false);
    expect(tauri.updateSetting).toHaveBeenCalledWith("groq_api_key", "");
  });
});
