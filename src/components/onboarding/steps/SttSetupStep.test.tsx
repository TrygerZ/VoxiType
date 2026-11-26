import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { t } from "../../../lib/i18n";
import * as tauri from "../../../lib/tauri";
import { SttSetupStep } from "./SttSetupStep";

describe("SttSetupStep", () => {
  const defaultProps = {
    step: "stt_setup" as const,
    currentStepIdx: 3,
    t,
    error: "",
    busy: false,
    sttEngine: "groq" as const,
    sttLanguage: "auto",
    apiKey: "",
    whisperBinary: "",
    whisperModel: "",
    whisperThreads: 4,
    onEngineChange: vi.fn(),
    onLanguageChange: vi.fn(),
    onApiKeyChange: vi.fn(),
    onSave: vi.fn(),
    onThreadsChange: vi.fn(),
    onBinaryChange: vi.fn(),
    onModelChange: vi.fn(),
    onBack: vi.fn(),
    onSkip: vi.fn(),
  };

  it("exposes the language select with an accessible name", () => {
    render(<SttSetupStep {...defaultProps} />);

    expect(
      screen.getByRole("combobox", { name: t("onboarding.stt.language") }),
    ).toBeInTheDocument();
  });

  it("shows existing saved key badge when groqKeySet is true, tests with null, and wraps whisper.cpp guide in details", async () => {
    const user = userEvent.setup();
    vi.mocked(tauri.testGroqApi).mockResolvedValueOnce(undefined);

    const { rerender } = render(<SttSetupStep {...defaultProps} groqKeySet />);

    expect(
      screen.getByText(/Existing key saved - leave blank to keep/i),
    ).toBeInTheDocument();
    const testBtn = screen.getByRole("button", { name: t("onboarding.stt.test") });
    expect(testBtn).toBeEnabled();
    await user.click(testBtn);
    await waitFor(() => {
      expect(tauri.testGroqApi).toHaveBeenCalledWith(null);
    });

    rerender(<SttSetupStep {...defaultProps} sttEngine="whisper_cpp" />);
    const details = document.querySelector("details");
    expect(details).not.toBeNull();
    expect(details).not.toHaveAttribute("open");
  });
});
