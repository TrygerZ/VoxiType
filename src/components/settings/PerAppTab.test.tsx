import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PerAppTab } from "./PerAppTab";
import * as tauri from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";

describe("PerAppTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tauri.getPerAppModes).mockResolvedValue([
      {
        id: 1,
        app_process_name: "slack.exe",
        app_display_name: "Slack",
        mode_id: "message",
      },
    ]);
    vi.mocked(tauri.listRunningApps).mockResolvedValue(["chrome.exe", "code.exe"]);
    vi.mocked(tauri.getActiveApp).mockResolvedValue("obsidian");
    useSettingsStore.setState({
      settings: { language: "en", per_app_mode: true },
      loaded: true,
      error: null,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("runs a 3s countdown on Detect (3s) before capturing active app and loads running app suggestions", async () => {
    vi.useFakeTimers();
    render(<PerAppTab />);

    await act(async () => {});
    expect(tauri.listRunningApps).toHaveBeenCalled();

    const detectBtn = screen.getByRole("button", { name: "Detect (3s)" });
    fireEvent.click(detectBtn);

    expect(tauri.getActiveApp).not.toHaveBeenCalled();
    expect(screen.getByText(/in 3s/)).toBeInTheDocument();

    await act(async () => {
      vi.advanceTimersByTime(2000);
    });
    expect(tauri.getActiveApp).not.toHaveBeenCalled();

    await act(async () => {
      vi.advanceTimersByTime(1000);
    });
    expect(tauri.getActiveApp).toHaveBeenCalledTimes(1);
    expect(screen.getByDisplayValue("obsidian")).toBeInTheDocument();
  });

  it("updates an existing rule mode inline via setPerAppMode without raw invoke", async () => {
    render(<PerAppTab />);

    const modeSelect = await screen.findByRole("combobox", {
      name: "Mode for slack.exe",
    });
    fireEvent.change(modeSelect, { target: { value: "email" } });

    await waitFor(() => {
      expect(tauri.setPerAppMode).toHaveBeenCalledWith({
        id: 1,
        app_process_name: "slack.exe",
        app_display_name: "Slack",
        mode_id: "email",
      });
    });
  });
});
