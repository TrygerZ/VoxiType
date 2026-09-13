import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { HomeView } from "./HomeView";
import * as tauri from "../../lib/tauri";
import { useAppStore } from "../../stores/appStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useHistoryStore } from "../../stores/historyStore";
import { useStatsStore } from "../../stores/statsStore";
import { useToastStore } from "../../stores/toastStore";

describe("HomeView recording actions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useToastStore.setState({ toasts: [] });
    useAppStore.setState({
      state: "idle",
      audioLevel: 0,
      durationSec: 0,
      wordCount: null,
      errorMessage: null,
    });
    useSettingsStore.setState({
      settings: { language: "en" },
      loaded: true,
      error: null,
    });
    useHistoryStore.setState({
      items: [],
      loading: false,
      error: null,
      query: "",
    });
    useStatsStore.setState({
      totals: { total_words: 0, total_duration_ms: 0, total_sessions: 0 },
      loaded: false,
      error: null,
    });
  });

  it("surfaces start recording failure as toast", async () => {
    vi.mocked(tauri.startRecording).mockRejectedValueOnce(
      new Error("microphone permission denied"),
    );

    render(<HomeView />);

    const user = userEvent.setup();
    const micButton = screen.getByTitle(/start dictat|mulai mendikte/i);

    await user.click(micButton);

    await waitFor(() => {
      const toasts = useToastStore.getState().toasts;
      expect(toasts.length).toBe(1);
      expect(toasts[0].message).toBe("microphone permission denied");
      expect(toasts[0].type).toBe("error");
    });
  });

  it("surfaces stop recording failure as toast", async () => {
    useAppStore.setState({ state: "recording" });
    vi.mocked(tauri.stopRecording).mockRejectedValueOnce(
      new Error("failed to stop pipeline"),
    );

    render(<HomeView />);

    const user = userEvent.setup();
    const micButton = screen.getByTitle(/stop dictat|selesai mendikte/i);

    await user.click(micButton);

    await waitFor(() => {
      const toasts = useToastStore.getState().toasts;
      expect(toasts.length).toBe(1);
      expect(toasts[0].message).toBe("failed to stop pipeline");
      expect(toasts[0].type).toBe("error");
    });
  });

  it("avoids duplicate recording invocations on rapid toggles", async () => {
    let resolveFirst!: () => void;
    const pendingPromise = new Promise<void>((resolve) => {
      resolveFirst = resolve;
    });

    vi.mocked(tauri.startRecording).mockImplementationOnce(() => pendingPromise);

    render(<HomeView />);

    const user = userEvent.setup();
    const micButton = screen.getByTitle(/start dictat|mulai mendikte/i);

    await user.click(micButton);
    await user.click(micButton);
    await user.click(micButton);

    expect(tauri.startRecording).toHaveBeenCalledTimes(1);

    resolveFirst();
  });
});
