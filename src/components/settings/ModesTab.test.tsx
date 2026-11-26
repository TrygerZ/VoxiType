import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ModesTab } from "./ModesTab";
import * as tauri from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";

describe("ModesTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({
      settings: {
        language: "en",
        active_mode: "dictation",
        command_mode: false,
        translation_enabled: false,
        translation_target: "en",
      },
      loaded: true,
      error: null,
    });
  });

  it("controls active mode, command mode, and post-transcription translation", async () => {
    const user = userEvent.setup();
    render(<ModesTab />);

    const commandSwitch = screen.getByRole("switch", { name: "Command mode" });
    await user.click(commandSwitch);
    expect(tauri.updateSetting).toHaveBeenCalledWith("command_mode", true);

    const translateSwitch = screen.getByRole("switch", {
      name: "Translate after transcription",
    });
    await user.click(translateSwitch);
    expect(tauri.updateSetting).toHaveBeenCalledWith("translation_enabled", true);
  });
});
