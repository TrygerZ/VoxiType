import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AboutTab } from "./AboutTab";
import * as tauri from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";

describe("AboutTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tauri.getAppInfo).mockResolvedValue({
      name: "VoxiType",
      version: "0.7.1",
      tauri: "2.x",
      platform: "windows",
    });
    useSettingsStore.setState({
      settings: { language: "en", stt_engine: "groq", llm_engine: "ollama", theme: "dark" },
      loaded: true,
      error: null,
    });
  });

  it("opens release URL via openUrl when update is available and copies diagnostics", async () => {
    vi.mocked(tauri.checkUpdates).mockResolvedValueOnce({
      available: true,
      current_version: "0.7.1",
      latest_version: "0.8.0",
      notes: "",
      url: "https://github.com/voxitype/releases/v0.8.0",
    });

    const user = userEvent.setup();
    render(<AboutTab />);
    const writeText = vi
      .spyOn(window.navigator.clipboard, "writeText")
      .mockResolvedValue(undefined);

    expect(await screen.findByText("v0.7.1")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Check for updates" }));
    const releaseBtn = await screen.findByRole("button", {
      name: /Update available: v0\.8\.0/,
    });
    await user.click(releaseBtn);
    expect(tauri.openUrl).toHaveBeenCalledWith(
      "https://github.com/voxitype/releases/v0.8.0",
    );

    await user.click(screen.getByRole("button", { name: "Copy diagnostics" }));
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith(
        expect.stringContaining("0.7.1"),
      );
    });
  });
});
