import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { OnboardingFlow } from "./OnboardingFlow";
import * as tauri from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";

describe("OnboardingFlow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({ settings: {}, loaded: true, error: null });
  });

  it("renders the welcome step", () => {
    render(<OnboardingFlow onComplete={vi.fn()} />);

    expect(screen.getByRole("heading", { name: "Welcome to VoxiType" })).toBeInTheDocument();
  });

  it("opens quick settings from welcome", async () => {
    const user = userEvent.setup();
    render(<OnboardingFlow onComplete={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Start Setup" }));

    expect(screen.getByRole("heading", { name: "Quick Settings" })).toBeInTheDocument();
  });

  it("calls onComplete when welcome is skipped", async () => {
    const user = userEvent.setup();
    const onComplete = vi.fn();
    render(<OnboardingFlow onComplete={onComplete} />);

    await user.click(screen.getByRole("button", { name: "Skip" }));

    expect(onComplete).toHaveBeenCalledOnce();
  });

  it("dispatches only one save when the primary button is double-clicked", async () => {
    const user = userEvent.setup();
    // Keep the first update pending so the step does not advance and the
    // button stays mounted between the two clicks.
    vi.mocked(tauri.updateSetting).mockReturnValueOnce(new Promise(() => {}));

    render(<OnboardingFlow onComplete={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Start Setup" }));

    const continueButton = screen.getByRole("button", { name: "Continue" });
    await user.click(continueButton);
    await user.click(continueButton);

    expect(tauri.updateSetting).toHaveBeenCalledTimes(1);
    expect(tauri.updateSetting).toHaveBeenCalledWith("language", "en");
  });
});
