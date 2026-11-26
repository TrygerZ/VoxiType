import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import { FileTranscriptionPanel } from "./FileTranscriptionPanel";
import { formatElapsed } from "./FileQueueItem";
import * as tauri from "../../lib/tauri";
import { useFileTranscriptionStore, type FileItem } from "../../stores/fileTranscriptionStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useToastStore } from "../../stores/toastStore";

const MEETING = "C:\\audio\\meeting.mp3";
const LECTURE = "C:\\audio\\lecture.wav";

const item = (path: string, patch: Partial<FileItem> = {}): FileItem => ({
  path,
  status: "queued",
  progress: null,
  result: null,
  error: null,
  startedAt: null,
  finishedAt: null,
  ...patch,
});

const done = (path: string, text: string, startedAt: number, finishedAt: number) =>
  item(path, {
    status: "done",
    startedAt,
    finishedAt,
    result: { id: path, text, word_count: 2, duration_ms: 1000, stt_issue: null, llm_issue: null },
  });

describe("FileTranscriptionPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useToastStore.setState({ toasts: [] });
    useSettingsStore.setState({ settings: { groq_api_key_set: true, llm_engine: "groq" } });
    useFileTranscriptionStore.setState({
      items: [item(MEETING)],
      options: null,
      picking: false,
      status: "idle",
      cancelled: false,
      exportFormat: "txt",
      exporting: false,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows the LLM engine picker only while cleanup is enabled", () => {
    render(<FileTranscriptionPanel />);
    expect(screen.queryByLabelText("LLM engine")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("switch", { name: "Clean up with LLM" }));

    const engine = screen.getByLabelText("LLM engine");
    expect(engine).toHaveValue("groq");
    expect(engine).toBeEnabled();
  });

  it("blocks starting when the chosen engine is not set up", () => {
    useSettingsStore.setState({ settings: { groq_api_key_set: false } });
    render(<FileTranscriptionPanel />);

    expect(screen.getByRole("alert")).toHaveTextContent("API key");
    expect(screen.getByRole("button", { name: /Start/ })).toBeDisabled();
  });

  it("lists every queued file, shows folder path, and exports only finished ones", () => {
    useFileTranscriptionStore.setState({
      items: [done(MEETING, "halo dunia", 0, 65_000), item(LECTURE)],
    });
    render(<FileTranscriptionPanel />);

    expect(screen.getByText("2 / 20 files")).toBeInTheDocument();
    expect(screen.getByText("meeting.mp3")).toBeInTheDocument();
    expect(screen.getByText("lecture.wav")).toBeInTheDocument();
    expect(screen.getByText("C:\\audio")).toBeInTheDocument();
    expect(screen.getByText(/01:05/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Export meeting.mp3" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Export lecture.wav" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Export all \(1\)/ })).toBeEnabled();
  });

  it("ticks the timer of the running file and locks queue mutations", () => {
    vi.useFakeTimers();
    vi.setSystemTime(10_000);
    useFileTranscriptionStore.setState({
      items: [item(MEETING, { status: "running", startedAt: 10_000 })],
      status: "running",
    });
    render(<FileTranscriptionPanel />);
    expect(screen.getByText("00:00")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel batch" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Clear" })).not.toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(3_000);
    });

    expect(screen.getByText("00:03")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Remove meeting.mp3" })).not.toBeInTheDocument();
  });

  it("supports + Add files... and Clear when queue has items", async () => {
    vi.mocked(tauri.pickAudioFiles).mockResolvedValueOnce([LECTURE]);
    render(<FileTranscriptionPanel />);

    fireEvent.click(screen.getByRole("button", { name: "+ Add files..." }));
    await waitFor(() => {
      expect(useFileTranscriptionStore.getState().items).toHaveLength(2);
    });

    fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    expect(useFileTranscriptionStore.getState().items).toHaveLength(0);
  });

  it("highlights dropzone on hover and registers dropped audio paths via Tauri webview event", async () => {
    let handler: ((event: { payload: { type: string; paths?: string[] } }) => void) | undefined;
    const webview = getCurrentWebview();
    vi.mocked(webview.onDragDropEvent).mockImplementation(async (cb) => {
      handler = cb as typeof handler;
      return () => undefined;
    });
    vi.mocked(tauri.registerDroppedAudioFiles).mockResolvedValueOnce([LECTURE]);

    useFileTranscriptionStore.setState({ items: [] });
    render(<FileTranscriptionPanel />);

    await waitFor(() => {
      expect(handler).toBeDefined();
    });

    const dropzone = screen.getByTestId("file-dropzone");
    act(() => {
      handler?.({ payload: { type: "enter", paths: [LECTURE] } });
    });
    expect(dropzone.className).toContain("border-vx-accent-ember");

    await act(async () => {
      handler?.({ payload: { type: "drop", paths: [LECTURE] } });
    });
    expect(tauri.registerDroppedAudioFiles).toHaveBeenCalledWith([LECTURE]);
    await waitFor(() => {
      expect(useFileTranscriptionStore.getState().items.map((i) => i.path)).toContain(LECTURE);
    });
  });

  it("renders Partial STT badge, warning banner, and Retry button for partial transcripts", () => {
    useFileTranscriptionStore.setState({
      items: [
        item(MEETING, {
          status: "done",
          result: {
            id: "partial-1",
            text: "Preserved partial text",
            word_count: 3,
            duration_ms: 4000,
            stt_issue: { count: 1, total: 2, reason: "corrupt AAC frame" },
            llm_issue: null,
          },
        }),
      ],
    });
    const retrySpy = vi.fn().mockResolvedValue(undefined);
    useFileTranscriptionStore.setState({ retry: retrySpy });

    render(<FileTranscriptionPanel />);

    expect(screen.getByText("Partial STT")).toBeInTheDocument();
    const banner = screen.getByRole("alert");
    expect(banner.className).toContain("bg-vx-semantic-warning-soft");

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(retrySpy).toHaveBeenCalledWith(MEETING);
  });

  it("fires a success toast with destination folder after batch export", async () => {
    useFileTranscriptionStore.setState({
      items: [done(MEETING, "halo dunia", 0, 5000)],
    });
    vi.mocked(tauri.pickExportDirectory).mockResolvedValueOnce("D:\\Exports");
    vi.mocked(tauri.exportTranscripts).mockResolvedValueOnce(["D:\\Exports\\meeting.txt"]);

    render(<FileTranscriptionPanel />);
    fireEvent.click(screen.getByRole("button", { name: /Export all \(1\)/ }));

    await waitFor(() => {
      const toasts = useToastStore.getState().toasts;
      expect(toasts).toHaveLength(1);
      expect(toasts[0].type).toBe("success");
      expect(toasts[0].message).toContain("1");
      expect(toasts[0].description).toBe("D:\\Exports");
    });
  });

  it("formats elapsed time with hours only when needed", () => {
    expect(formatElapsed(59_999)).toBe("00:59");
    expect(formatElapsed(3_600_000 + 61_000)).toBe("1:01:01");
  });
});
