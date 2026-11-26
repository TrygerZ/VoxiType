import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AudioTab } from "./AudioTab";
import * as tauri from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";

describe("AudioTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tauri.getMicrophones).mockResolvedValue([
      { id: "mic-1", name: "Built-in Mic", is_default: true },
    ]);
    useSettingsStore.setState({
      settings: { language: "en", mic_device: "mic-1", sound_cues: false },
      loaded: true,
      error: null,
    });
  });

  it("refreshes devices, starts and stops mic meter, and toggles sound cues", async () => {
    const user = userEvent.setup();
    const { unmount } = render(<AudioTab />);

    await waitFor(() => {
      expect(tauri.getMicrophones).toHaveBeenCalledTimes(1);
      expect(tauri.setMicMeter).toHaveBeenCalledWith(true, "mic-1");
    });

    vi.mocked(tauri.getMicrophones).mockResolvedValueOnce([
      { id: "mic-1", name: "Built-in Mic", is_default: false },
      { id: "mic-2", name: "Shure MV7", is_default: true },
    ]);
    await user.click(screen.getByRole("button", { name: "Refresh" }));
    expect(await screen.findByRole("option", { name: "Shure MV7" })).toBeInTheDocument();

    await user.click(screen.getByRole("switch", { name: "Sound cues" }));
    expect(tauri.updateSetting).toHaveBeenCalledWith("sound_cues", true);

    unmount();
    expect(tauri.setMicMeter).toHaveBeenLastCalledWith(false, null);
  });
});
