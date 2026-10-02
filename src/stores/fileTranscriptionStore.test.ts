import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import { useFileTranscriptionStore } from "./fileTranscriptionStore";

const OPTIONS = {
  stt_engine: "groq",
  llm_engine: null,
  apply_dictionary: true,
} as const;

describe("fileTranscriptionStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useFileTranscriptionStore.setState({
      path: "C:\\audio\\meeting.mp3",
      status: "idle",
      progress: null,
      result: null,
      error: null,
    });
  });

  it("sends the picked path and stores the result", async () => {
    const result = { id: "h1", text: "halo", word_count: 1, duration_ms: 900 };
    vi.mocked(tauri.transcribeFile).mockResolvedValueOnce(result);

    await useFileTranscriptionStore.getState().start(OPTIONS);

    expect(tauri.transcribeFile).toHaveBeenCalledWith({
      ...OPTIONS,
      path: "C:\\audio\\meeting.mp3",
    });
    const state = useFileTranscriptionStore.getState();
    expect(state.status).toBe("done");
    expect(state.result).toEqual(result);
  });

  it("records the backend error message on failure", async () => {
    vi.mocked(tauri.transcribeFile).mockRejectedValueOnce({
      code: "InvalidInput",
      message: "Audio file is longer than the 60-minute limit",
    });

    await useFileTranscriptionStore.getState().start(OPTIONS);

    const state = useFileTranscriptionStore.getState();
    expect(state.status).toBe("error");
    expect(state.error).toBe("Audio file is longer than the 60-minute limit");
  });

  it("does not start without a file or while running", async () => {
    useFileTranscriptionStore.setState({ path: null });
    await useFileTranscriptionStore.getState().start(OPTIONS);
    useFileTranscriptionStore.setState({ path: "a.mp3", status: "running" });
    await useFileTranscriptionStore.getState().start(OPTIONS);

    expect(tauri.transcribeFile).not.toHaveBeenCalled();
  });
});
