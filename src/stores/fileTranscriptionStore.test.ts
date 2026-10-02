import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import { useFileTranscriptionStore } from "./fileTranscriptionStore";

const OPTIONS = {
  stt_engine: "groq",
  language: "auto",
  llm_engine: null,
  apply_dictionary: true,
} as const;

const PATH = "C:\\audio\\meeting.mp3";

describe("fileTranscriptionStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useFileTranscriptionStore.setState({
      path: PATH,
      options: { ...OPTIONS },
      picking: false,
      status: "idle",
      progress: null,
      result: null,
      error: null,
      cancelled: false,
    });
  });

  it("sends the picked path with the stored options and keeps the result", async () => {
    const result = {
      id: "h1",
      text: "halo",
      word_count: 1,
      duration_ms: 900,
      stt_issue: null,
      llm_issue: null,
    };
    vi.mocked(tauri.transcribeFile).mockResolvedValueOnce(result);

    await useFileTranscriptionStore.getState().start();

    expect(tauri.transcribeFile).toHaveBeenCalledWith({ ...OPTIONS, path: PATH });
    const state = useFileTranscriptionStore.getState();
    expect(state.status).toBe("done");
    expect(state.result).toEqual(result);
    expect(tauri.getHistory).toHaveBeenCalled();
  });

  it("records the backend error message on failure", async () => {
    vi.mocked(tauri.transcribeFile).mockRejectedValueOnce({
      code: "InvalidInput",
      message: "Audio file is longer than the 60-minute limit",
    });

    await useFileTranscriptionStore.getState().start();

    const state = useFileTranscriptionStore.getState();
    expect(state.status).toBe("error");
    expect(state.error).toBe("Audio file is longer than the 60-minute limit");
  });

  it("treats a user cancel as a neutral outcome, not an error", async () => {
    vi.mocked(tauri.transcribeFile).mockRejectedValueOnce({
      code: "Cancelled",
      message: "File transcription cancelled",
    });

    await useFileTranscriptionStore.getState().start();

    const state = useFileTranscriptionStore.getState();
    expect(state.status).toBe("idle");
    expect(state.cancelled).toBe(true);
    expect(state.error).toBeNull();
  });

  it("does not start without a file or while running", async () => {
    useFileTranscriptionStore.setState({ path: null });
    await useFileTranscriptionStore.getState().start();
    useFileTranscriptionStore.setState({ path: PATH, status: "running" });
    await useFileTranscriptionStore.getState().start();

    expect(tauri.transcribeFile).not.toHaveBeenCalled();
  });

  it("opens only one file picker for repeated clicks", async () => {
    let resolvePick!: (path: string | null) => void;
    vi.mocked(tauri.pickAudioFile).mockImplementationOnce(
      () => new Promise((resolve) => (resolvePick = resolve)),
    );

    const first = useFileTranscriptionStore.getState().pick();
    await useFileTranscriptionStore.getState().pick();
    await useFileTranscriptionStore.getState().pick();
    resolvePick("D:\\new.wav");
    await first;

    expect(tauri.pickAudioFile).toHaveBeenCalledTimes(1);
    const state = useFileTranscriptionStore.getState();
    expect(state.path).toBe("D:\\new.wav");
    expect(state.picking).toBe(false);
  });

  it("locks options while a job is running", () => {
    useFileTranscriptionStore.setState({ status: "running" });
    useFileTranscriptionStore.getState().setOptions({ stt_engine: "whisper_cpp" });

    expect(useFileTranscriptionStore.getState().options?.stt_engine).toBe("groq");
  });
});
