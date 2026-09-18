import { act, render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { t } from "../../../lib/i18n";
import * as tauri from "../../../lib/tauri";
import type { DeviceInfo } from "../../../types/app";
import { MicrophoneStep } from "./MicrophoneStep";

vi.mock("../../../lib/tauri", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../../lib/tauri")>();
  return {
    ...actual,
    getMicrophones: vi.fn(),
  };
});

describe("MicrophoneStep", () => {
  const defaultProps = {
    step: "microphone" as const,
    currentStepIdx: 2,
    t,
    selectedDevice: "",
    onDeviceChange: vi.fn(),
    onBack: vi.fn(),
    onContinue: vi.fn(),
    onSkip: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("does not overwrite user choice when getMicrophones resolves after manual selection", async () => {
    let resolveMics!: (devices: DeviceInfo[]) => void;
    const pendingPromise = new Promise<DeviceInfo[]>((resolve) => {
      resolveMics = resolve;
    });

    vi.mocked(tauri.getMicrophones).mockReturnValueOnce(pendingPromise);

    const onDeviceChange = vi.fn();
    const { rerender } = render(
      <MicrophoneStep
        {...defaultProps}
        selectedDevice=""
        onDeviceChange={onDeviceChange}
      />,
    );

    // User chooses a microphone manually while enumeration is still in flight
    rerender(
      <MicrophoneStep
        {...defaultProps}
        selectedDevice="user-selected-mic"
        onDeviceChange={onDeviceChange}
      />,
    );

    // Enumeration completes late with a default device
    await act(async () => {
      resolveMics([
        { id: "default-mic", name: "Default Microphone", is_default: true },
        { id: "other-mic", name: "Other Microphone", is_default: false },
      ]);
      await pendingPromise;
    });

    expect(onDeviceChange).not.toHaveBeenCalledWith("default-mic");
  });

  it("selects default microphone when user has not selected any device", async () => {
    const onDeviceChange = vi.fn();
    vi.mocked(tauri.getMicrophones).mockResolvedValueOnce([
      { id: "default-mic", name: "Default Microphone", is_default: true },
    ]);

    render(
      <MicrophoneStep
        {...defaultProps}
        selectedDevice=""
        onDeviceChange={onDeviceChange}
      />,
    );

    await act(async () => {
      await Promise.resolve();
    });

    expect(onDeviceChange).toHaveBeenCalledWith("default-mic");
  });
});
