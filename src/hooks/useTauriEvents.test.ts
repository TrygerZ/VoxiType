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
        injected: true,
        app_context: "winword",
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
        injected: true,
        app_context: "winword",
      });
    });

    expect(useAppStore.getState().wordCount).toBe(4);
    expect(tauri.getHistory).toHaveBeenCalled();
    expect(tauri.getUsageStats).toHaveBeenCalled();
  });

  it("unlistens the surviving registrations when one listen rejects", async () => {
    const stateUnlisten = vi.fn();
    const completeUnlisten = vi.fn();
    vi.mocked(tauri.onEvent).mockImplementation((event) => {
      if (event === "state_changed") {
        return Promise.reject(new Error("listen failed"));
      }
      return Promise.resolve(event === "transcription_complete"
        ? completeUnlisten
        : stateUnlisten);
    });

    const { unmount } = renderHook(() => useTauriEvents());
    unmount();

    await act(async () => {
      await Promise.resolve();
    });

    expect(completeUnlisten).toHaveBeenCalledTimes(1);
    expect(stateUnlisten).toHaveBeenCalledTimes(1);
  });
});
