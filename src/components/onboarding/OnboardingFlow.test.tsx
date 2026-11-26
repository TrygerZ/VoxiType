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

  it("navigates back from STT setup to Microphone step and surfaces mic save error", async () => {
    const user = userEvent.setup();
    vi.mocked(tauri.getMicrophones).mockResolvedValue([
      { id: "mic-1", name: "Studio Mic", is_default: true },
    ]);
    useSettingsStore.setState({
      settings: { language: "en", stt_engine: "groq" },
      loaded: true,
      error: null,
    });

    render(<OnboardingFlow onComplete={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Start Setup" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByRole("heading", { name: "Choose a microphone" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Save & continue" }));
    expect(screen.getByRole("heading", { name: "Choose transcription setup" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Back to Microphone/i }));
    expect(screen.getByRole("heading", { name: "Choose a microphone" })).toBeInTheDocument();

    vi.mocked(tauri.updateSetting).mockRejectedValueOnce(new Error("Mic write failed"));
    await user.click(screen.getByRole("button", { name: "Save & continue" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Mic write failed");
  });

  it("resumes from persisted onboarding_resume_step, allows Back on CompleteStep, and clears resume state on finish", async () => {
    const user = userEvent.setup();
    const onComplete = vi.fn();
    useSettingsStore.setState({
      settings: {
        language: "en",
        onboarding_resume_step: "hotkey",
        mic_device: "Shure MV7",
        stt_engine: "groq",
        stt_language: "auto",
      },
      loaded: true,
      error: null,
    });

    render(<OnboardingFlow onComplete={onComplete} />);

    expect(screen.getByRole("heading", { name: "Hotkey" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByRole("heading", { name: "Try your dictation" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Skip" }));
    expect(screen.getByRole("heading", { name: "All set!" })).toBeInTheDocument();
    expect(screen.getByText(/Active configuration summary/i)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByRole("heading", { name: "Try your dictation" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Skip" }));
    await user.click(screen.getByRole("button", { name: "Start using" }));

    expect(tauri.updateSetting).toHaveBeenCalledWith("onboarding_resume_step", "");
    expect(tauri.updateSetting).toHaveBeenCalledWith("onboarding_completed", true);
    expect(onComplete).toHaveBeenCalledOnce();
  });

  it("switches appearance theme in QuickSettingsStep without duplicating sound cues label", async () => {
    const user = userEvent.setup();
    render(<OnboardingFlow onComplete={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Start Setup" }));

    const lightBtn = screen.getByRole("button", { name: "Light" });
    await user.click(lightBtn);
    expect(tauri.updateSetting).toHaveBeenCalledWith("theme", "light");

    const switches = screen.getAllByRole("switch", { name: "Recording sounds" });
    expect(switches).toHaveLength(1);
    expect(screen.getAllByText("Recording sounds")).toHaveLength(1);
  });
});
