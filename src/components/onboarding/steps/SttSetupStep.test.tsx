import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { t } from "../../../lib/i18n";
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
});
