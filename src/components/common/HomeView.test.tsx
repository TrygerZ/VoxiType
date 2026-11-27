import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { HomeView } from "./HomeView";
import * as tauri from "../../lib/tauri";
import { setNavigator } from "../../lib/navigate";
import { useAppStore } from "../../stores/appStore";
import { useDictionaryStore } from "../../stores/dictionaryStore";
import { useHistoryStore } from "../../stores/historyStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useSnippetStore } from "../../stores/snippetStore";
import { useStatsStore } from "../../stores/statsStore";
import { useToastStore } from "../../stores/toastStore";

describe("HomeView recording actions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useToastStore.setState({ toasts: [] });
    useAppStore.setState({
      state: "idle",
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
    useDictionaryStore.setState({ entries: [], loading: false, error: null });
    useSnippetStore.setState({ snippets: [], loading: false, error: null });
    useStatsStore.setState({
      totals: { total_words: 0, total_duration_ms: 0, total_sessions: 0 },
      loaded: false,
      error: null,
    });
  });

  it("labels the translation target options consistently", () => {
    useSettingsStore.setState({
      settings: { language: "en", translation_enabled: true },
      loaded: true,
      error: null,
    });

    render(<HomeView />);

    expect(
      screen.getByRole("option", { name: "English" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("option", { name: "Bahasa Indonesia" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: "Indonesia" }),
    ).not.toBeInTheDocument();
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

  it("names the injection target after a session reaches the app", () => {
    useAppStore.setState({
      wordCount: 4,
      outcome: { injected: true, appContext: "winword" },
    });

    render(<HomeView />);

    expect(screen.getByText("Inserted into winword")).toHaveAttribute(
      "role",
      "status",
    );
  });

  it("says the text stayed in History when injection failed", () => {
    useAppStore.setState({
      wordCount: 4,
      outcome: { injected: false, appContext: "winword" },
    });

    render(<HomeView />);

    expect(
      screen.getByText("The text did not reach the target app. It is saved in History."),
    ).toHaveAttribute("role", "status");
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

describe("HomeView v2 redesign & navigation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setNavigator(null);
    useToastStore.setState({ toasts: [] });
    useAppStore.setState({
      state: "idle",
      durationSec: 0,
      wordCount: null,
      outcome: null,
      errorMessage: null,
    });
    useSettingsStore.setState({
      settings: {
        language: "en",
        groq_api_key_set: true,
        mic_device: "Shure MV7 (USB Audio)",
        stt_engine: "groq",
        llm_engine: "groq",
        stt_language: "auto",
        active_mode: "dictation",
        hotkey: { key: "Ctrl+Space", mode: "ptt" },
      },
      loaded: true,
      error: null,
    });
    useDictionaryStore.setState({
      entries: [
        {
          id: "d1",
          word: "VoxiType",
          pronunciation: null,
          category: "custom",
          replacement: null,
          language: "en",
          usage_count: 3,
          is_active: true,
        },
        {
          id: "d2",
          word: "Tauri",
          pronunciation: null,
          category: "custom",
          replacement: null,
          language: "en",
          usage_count: 1,
          is_active: false,
        },
      ],
      loading: false,
      error: null,
    });
    useSnippetStore.setState({
      snippets: [
        {
          id: "s1",
          name: "signoff",
          trigger_phrase: "sign off",
          content: "Best regards,",
          category: null,
          mode: null,
          usage_count: 2,
          is_active: true,
        },
      ],
      loading: false,
      error: null,
    });
    useStatsStore.setState({
      totals: { total_words: 28600, total_duration_ms: 12_240_000, total_sessions: 312 },
      loaded: true,
      error: null,
    });
    useHistoryStore.setState({
      items: [
        {
          id: "rec-1",
          text_raw: "Let us align on the API schema",
          text_formatted: "Let's align on the API schema before Friday.",
          mode: "message",
          source_lang: "en",
          duration_ms: 6000,
          word_count: 8,
          character_count: 44,
          stt_engine: "groq",
          llm_engine: "groq",
          app_context: "Slack",
          created_at: new Date().toISOString(),
          is_pinned: false,
        },
      ],
      loading: false,
      error: null,
      query: "",
    });
  });

  it("navigates to Settings Audio, Settings STT, Dictionary, Snippets, and History from quick links", async () => {
    const navSpy = vi.fn();
    setNavigator(navSpy);

    render(<HomeView />);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: /Shure MV7/i }));
    expect(navSpy).toHaveBeenCalledWith("settings", "audio");

    await user.click(screen.getByRole("button", { name: /Groq Whisper/i }));
    expect(navSpy).toHaveBeenCalledWith("settings", "stt");

    await user.click(screen.getByRole("button", { name: /terms/i }));
    expect(navSpy).toHaveBeenCalledWith("dictionary", undefined);

    await user.click(screen.getByRole("button", { name: /snippets/i }));
    expect(navSpy).toHaveBeenCalledWith("snippets", undefined);

    await user.click(screen.getByRole("button", { name: /View all in History/i }));
    expect(navSpy).toHaveBeenCalledWith("history", undefined);
  });

  it("updates stt_language via the 1-click Active Setup language switcher", async () => {
    render(<HomeView />);
    const user = userEvent.setup();

    const autoBtn = screen.getByRole("button", { name: /^Auto$/i });
    const idBtn = screen.getByRole("button", { name: /^ID$/i });
    expect(autoBtn).toHaveAttribute("aria-pressed", "true");
    expect(idBtn).toHaveAttribute("aria-pressed", "false");

    await user.click(idBtn);
    expect(tauri.updateSetting).toHaveBeenCalledWith("stt_language", "id");
  });

  it("renders Copy and Pin icon buttons for recent items and omits banned glass classes", async () => {
    const entry = {
      id: "rec-1",
      text_raw: "Let us align on the API schema",
      text_formatted: "Let's align on the API schema before Friday.",
      mode: "message",
      source_lang: "en",
      duration_ms: 6000,
      word_count: 8,
      character_count: 44,
      stt_engine: "groq",
      llm_engine: "groq",
      app_context: "Slack",
      created_at: new Date().toISOString(),
      is_pinned: false,
    };
    vi.mocked(tauri.getHistory).mockResolvedValueOnce([entry]);
    useHistoryStore.setState({ items: [entry] });

    const { container } = render(<HomeView />);
    const user = userEvent.setup();
    const writeText = vi
      .spyOn(window.navigator.clipboard, "writeText")
      .mockResolvedValue(undefined);

    expect(container.querySelector(".backdrop-blur-md")).toBeNull();
    expect(container.querySelector(".rounded-3xl")).toBeNull();
    expect(container.querySelector(".blur-3xl")).toBeNull();
    expect(container.querySelector('img[src="/logo.png"]')).not.toBeNull();

    await user.click(screen.getByRole("button", { name: /^Copy$/i }));
    expect(writeText).toHaveBeenCalledWith(
      "Let's align on the API schema before Friday.",
    );

    await user.click(screen.getByRole("button", { name: /Pin transcription/i }));
    expect(tauri.pinHistory).toHaveBeenCalledWith("rec-1", true);
  });

  it("shows recording timer, Stop morph, and vx-mic-pulse ripples when recording is active", () => {
    useAppStore.setState({ state: "recording", durationSec: 14 });

    const { container } = render(<HomeView />);

    expect(screen.getByText("00:14")).toBeInTheDocument();
    expect(screen.getByText("Recording · 00:14")).toBeInTheDocument();
    expect(screen.getByTestId("mic-stop-icon")).toBeInTheDocument();
    expect(container.querySelectorAll(".vx-mic-pulse")).toHaveLength(2);
  });

  it("renders calibrated WpmHalfRing with computed WPM and sliding mode pill", () => {
    render(<HomeView />);

    const ring = screen.getByTestId("wpm-half-ring");
    expect(ring).toBeInTheDocument();
    expect(ring).toHaveTextContent("80");
    expect(ring).toHaveTextContent("WPM");
    expect(screen.getByText("Speaking pace")).toBeInTheDocument();
    expect(screen.getByTestId("mode-active-pill")).toBeInTheDocument();
  });

  it("shows orbiting hairline ring while processing and brief success flash on completion", () => {
    useAppStore.setState({ state: "processing" });
    const { rerender } = render(<HomeView />);

    const orbit = screen.getByTestId("mic-orbit-ring");
    expect(orbit.getAttribute("class")).toContain("vx-mic-orbit");

    useAppStore.setState({
      state: "idle",
      wordCount: 12,
      outcome: { injected: true, appContext: "Slack" },
    });
    rerender(<HomeView />);

    expect(screen.getByTestId("mic-success-icon")).toBeInTheDocument();
  });
});
