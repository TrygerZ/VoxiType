import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";

import { FileTranscriptionPanel } from "./FileTranscriptionPanel";
import { formatElapsed } from "./FileQueueItem";
import { useFileTranscriptionStore, type FileItem } from "../../stores/fileTranscriptionStore";
import { useSettingsStore } from "../../stores/settingsStore";

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
    expect(screen.getByRole("button", { name: /Start transcription/ })).toBeDisabled();
  });

  it("lists every queued file and exports only finished ones", () => {
    useFileTranscriptionStore.setState({
      items: [done(MEETING, "halo dunia", 0, 65_000), item(LECTURE)],
    });
    render(<FileTranscriptionPanel />);

    expect(screen.getByText("2 file(s) selected")).toBeInTheDocument();
    expect(screen.getByText("meeting.mp3")).toBeInTheDocument();
    expect(screen.getByText("lecture.wav")).toBeInTheDocument();
    expect(screen.getByText("01:05")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Export meeting.mp3" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Export lecture.wav" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Export all \(1\)/ })).toBeEnabled();
  });

  it("ticks the timer of the running file", () => {
    vi.useFakeTimers();
    vi.setSystemTime(10_000);
    useFileTranscriptionStore.setState({
      items: [item(MEETING, { status: "running", startedAt: 10_000 })],
      status: "running",
    });
    render(<FileTranscriptionPanel />);
    expect(screen.getByText("00:00")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(3_000);
    });

    expect(screen.getByText("00:03")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Remove meeting.mp3" })).not.toBeInTheDocument();
  });

  it("formats elapsed time with hours only when needed", () => {
    expect(formatElapsed(59_999)).toBe("00:59");
    expect(formatElapsed(3_600_000 + 61_000)).toBe("1:01:01");
  });
});
