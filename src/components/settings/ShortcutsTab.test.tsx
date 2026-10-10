import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ShortcutsTab } from "./ShortcutsTab";
import * as tauri from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";

async function applyHotkey() {
  const button = screen.getByRole("button", { name: "Apply hotkey" });
  fireEvent.click(button);
  await act(async () => {});
}

describe("ShortcutsTab applied status timer", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    useSettingsStore.setState({
      settings: {},
      loaded: true,
      error: null,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("restarts the reset window on a second apply so the status stays ok", async () => {
    render(<ShortcutsTab />);

    await applyHotkey();
    expect(screen.getByText("Applied")).toBeInTheDocument();

    // Second apply 1 s in; its window must replace the first timer's deadline.
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    await applyHotkey();

    // Past the first timer's original deadline (2 s) the status is still ok.
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.getByText("Applied")).toBeInTheDocument();

    // The second apply's own deadline still resets it.
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.queryByText("Applied")).toBeNull();
  });

  it("clears the pending timer on unmount", async () => {
    const clearSpy = vi.spyOn(globalThis, "clearTimeout");

    const { unmount } = render(<ShortcutsTab />);
    await applyHotkey();
    clearSpy.mockClear();

    unmount();

    expect(clearSpy).toHaveBeenCalled();
  });

  it("keeps the error when a failure follows a success within the reset window", async () => {
    render(<ShortcutsTab />);

    await applyHotkey();
    expect(screen.getByText("Applied")).toBeInTheDocument();

    // Second apply fails 500 ms in, while the first success timer is armed.
    act(() => {
      vi.advanceTimersByTime(500);
    });
    vi.mocked(tauri.setHotkey).mockRejectedValueOnce(new Error("boom"));
    await applyHotkey();
    expect(screen.getByText("boom")).toBeInTheDocument();

    // Past the first timer's deadline the error must still be shown.
    act(() => {
      vi.advanceTimersByTime(2000);
    });
    expect(screen.getByText("boom")).toBeInTheDocument();
  });
});

describe("ShortcutsTab apply failure", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({ settings: {}, loaded: true, error: null });
  });

  it("shows the error message when setHotkey rejects", async () => {
    vi.mocked(tauri.setHotkey).mockRejectedValueOnce(new Error("boom"));

    render(<ShortcutsTab />);
    await applyHotkey();

    expect(screen.getByText("boom")).toBeInTheDocument();
    expect(screen.queryByText("Applied")).toBeNull();
  });
});
