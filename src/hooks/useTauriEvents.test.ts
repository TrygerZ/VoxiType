import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import { useAppStore } from "../stores/appStore";
import { useTauriEvents } from "./useTauriEvents";

type EventHandler = (payload: unknown) => void;

describe("useTauriEvents", () => {
  let listeners: Record<string, EventHandler> = {};

  beforeEach(() => {
    vi.clearAllMocks();
    listeners = {};
    vi.mocked(tauri.onEvent).mockImplementation((event, handler) => {
      listeners[event] = handler as EventHandler;
      return Promise.resolve(() => undefined);
    });
    useAppStore.getState().reset();
  });

  it("updates wordCount without reloading history or stats when reloadData is false", () => {
    renderHook(() => useTauriEvents({ reloadData: false }));

    expect(listeners["transcription_complete"]).toBeDefined();

    act(() => {
      listeners["transcription_complete"]({
        id: 1,
        text: "hello overlay",
        word_count: 7,
        duration_ms: 1500,
      });
    });

    expect(useAppStore.getState().wordCount).toBe(7);
    expect(tauri.getHistory).not.toHaveBeenCalled();
    expect(tauri.getUsageStats).not.toHaveBeenCalled();
  });

  it("updates wordCount and triggers history and stats reloads by default", () => {
    renderHook(() => useTauriEvents());

    expect(listeners["transcription_complete"]).toBeDefined();

    act(() => {
      listeners["transcription_complete"]({
        id: 2,
        text: "hello main window",
        word_count: 4,
        duration_ms: 800,
      });
    });

    expect(useAppStore.getState().wordCount).toBe(4);
    expect(tauri.getHistory).toHaveBeenCalled();
    expect(tauri.getUsageStats).toHaveBeenCalled();
  });
});
