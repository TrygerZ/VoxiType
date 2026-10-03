import { beforeEach, describe, expect, it, vi } from "vitest";

import * as tauri from "../lib/tauri";
import { useFileTranscriptionStore, type FileItem } from "./fileTranscriptionStore";

const OPTIONS = {
  stt_engine: "groq",
  language: "auto",
  llm_engine: null,
  apply_dictionary: true,
} as const;

const FIRST = "C:\\audio\\meeting.mp3";
const SECOND = "C:\\audio\\lecture.wav";

const result = (text: string) => ({
  id: text,
  text,
  word_count: 1,
  duration_ms: 900,
  stt_issue: null,
  llm_issue: null,
});

const queued = (path: string): FileItem => ({
  path,
  status: "queued",
  progress: null,
  result: null,
  error: null,
  startedAt: null,
  finishedAt: null,
});

const items = () => useFileTranscriptionStore.getState().items;

describe("fileTranscriptionStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useFileTranscriptionStore.setState({
      items: [queued(FIRST), queued(SECOND)],
      options: { ...OPTIONS },
      picking: false,
      status: "idle",
      cancelled: false,
      exportFormat: "txt",
      exporting: false,
    });
  });

  it("transcribes queued files in order and times each one", async () => {
    vi.mocked(tauri.transcribeFile)
      .mockResolvedValueOnce(result("satu"))
      .mockResolvedValueOnce(result("dua"));

    await useFileTranscriptionStore.getState().start();

    expect(vi.mocked(tauri.transcribeFile).mock.calls.map(([req]) => req.path)).toEqual([FIRST, SECOND]);
    expect(items().map((i) => i.result?.text)).toEqual(["satu", "dua"]);
    for (const item of items()) {
      expect(item.status).toBe("done");
      expect(item.finishedAt).toBeGreaterThanOrEqual(item.startedAt ?? Infinity);
    }
    expect(useFileTranscriptionStore.getState().status).toBe("idle");
    expect(tauri.getHistory).toHaveBeenCalled();
  });

  it("records a failed file and moves on to the next", async () => {
    vi.mocked(tauri.transcribeFile)
      .mockRejectedValueOnce({ code: "InvalidInput", message: "No speech detected in the file" })
      .mockResolvedValueOnce(result("dua"));

    await useFileTranscriptionStore.getState().start();

    expect(items()[0]).toMatchObject({ status: "error", error: "No speech detected in the file" });
    expect(items()[1].status).toBe("done");
  });

  it("stops the whole queue on cancel and treats it as neutral", async () => {
    vi.mocked(tauri.transcribeFile).mockRejectedValueOnce({
      code: "Cancelled",
      message: "File transcription cancelled",
    });

    await useFileTranscriptionStore.getState().start();

    expect(tauri.transcribeFile).toHaveBeenCalledTimes(1);
    expect(items().map((i) => i.status)).toEqual(["cancelled", "queued"]);
    expect(items()[0].error).toBeNull();
    expect(useFileTranscriptionStore.getState().cancelled).toBe(true);
  });

  it("does not start the next file after a cancel between files", async () => {
    vi.mocked(tauri.transcribeFile).mockImplementationOnce(async () => {
      await useFileTranscriptionStore.getState().cancel();
      return result("satu");
    });

    await useFileTranscriptionStore.getState().start();

    expect(tauri.transcribeFile).toHaveBeenCalledTimes(1);
    expect(items().map((i) => i.status)).toEqual(["done", "queued"]);
    expect(useFileTranscriptionStore.getState().cancelled).toBe(true);
  });

  it("skips files that are already done when started again", async () => {
    useFileTranscriptionStore.setState({
      items: [{ ...queued(FIRST), status: "done", result: result("satu") }, queued(SECOND)],
    });
    vi.mocked(tauri.transcribeFile).mockResolvedValueOnce(result("dua"));

    await useFileTranscriptionStore.getState().start();

    expect(tauri.transcribeFile).toHaveBeenCalledTimes(1);
    expect(vi.mocked(tauri.transcribeFile).mock.calls[0][0].path).toBe(SECOND);
  });

  it("does not start while running or with an empty queue", async () => {
    useFileTranscriptionStore.setState({ status: "running" });
    await useFileTranscriptionStore.getState().start();
    useFileTranscriptionStore.setState({ status: "idle", items: [] });
    await useFileTranscriptionStore.getState().start();

    expect(tauri.transcribeFile).not.toHaveBeenCalled();
  });

  it("opens only one picker and replaces the queue with its result", async () => {
    let resolvePick!: (paths: string[]) => void;
    vi.mocked(tauri.pickAudioFiles).mockImplementationOnce(
      () => new Promise((resolve) => (resolvePick = resolve)),
    );

    const first = useFileTranscriptionStore.getState().pick();
    await useFileTranscriptionStore.getState().pick();
    resolvePick(["D:\\new.wav"]);
    await first;

    expect(tauri.pickAudioFiles).toHaveBeenCalledTimes(1);
    expect(items().map((i) => i.path)).toEqual(["D:\\new.wav"]);
    expect(useFileTranscriptionStore.getState().picking).toBe(false);
  });

  it("keeps the queue when the picker is closed", async () => {
    await useFileTranscriptionStore.getState().pick();
    expect(items()).toHaveLength(2);
  });

  it("locks options and the queue while a batch is running", () => {
    useFileTranscriptionStore.setState({ status: "running" });
    useFileTranscriptionStore.getState().setOptions({ stt_engine: "whisper_cpp" });
    useFileTranscriptionStore.getState().remove(FIRST);

    expect(useFileTranscriptionStore.getState().options?.stt_engine).toBe("groq");
    expect(items()).toHaveLength(2);
  });

  it("exports only finished files into the picked folder", async () => {
    useFileTranscriptionStore.setState({
      items: [{ ...queued(FIRST), status: "done", result: result("satu") }, queued(SECOND)],
      exportFormat: "pdf",
    });
    vi.mocked(tauri.pickExportDirectory).mockResolvedValueOnce("D:\\out");
    vi.mocked(tauri.exportTranscripts).mockResolvedValueOnce(["D:\\out\\meeting.pdf"]);

    const written = await useFileTranscriptionStore.getState().exportFiles([FIRST, SECOND]);

    expect(tauri.exportTranscripts).toHaveBeenCalledWith("D:\\out", "pdf", [
      { source_path: FIRST, text: "satu" },
    ]);
    expect(written).toEqual(["D:\\out\\meeting.pdf"]);
    expect(useFileTranscriptionStore.getState().exporting).toBe(false);
  });

  it("writes nothing when the export folder dialog is closed", async () => {
    useFileTranscriptionStore.setState({
      items: [{ ...queued(FIRST), status: "done", result: result("satu") }],
    });

    expect(await useFileTranscriptionStore.getState().exportFiles([FIRST])).toBeNull();
    expect(tauri.exportTranscripts).not.toHaveBeenCalled();
  });
});
