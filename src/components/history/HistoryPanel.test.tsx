import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { HistoryPanel } from "./HistoryPanel";
import { useHistoryStore } from "../../stores/historyStore";
import * as tauri from "../../lib/tauri";
import { t } from "../../lib/i18n";
import type { TranscriptionEntry } from "../../types/app";

const createMockItem = (
  id: string,
  text: string,
  patch: Partial<TranscriptionEntry> = {},
): TranscriptionEntry => ({
  id,
  text_raw: text,
  text_formatted: `${text}.`,
  source_lang: "en",
  mode: "dictation",
  stt_engine: "groq",
  duration_ms: 1200,
  word_count: 2,
  character_count: 12,
  is_pinned: false,
  created_at: "2026-01-01T00:00:00Z",
  ...patch,
});

describe("HistoryPanel load more", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useHistoryStore.setState({
      items: [createMockItem("h1", "hello")],
      loading: false,
      error: null,
      query: "",
      hasMore: false,
    });
  });

  it("shows the load more button when more pages exist", () => {
    useHistoryStore.setState({ hasMore: true });

    render(<HistoryPanel />);

    expect(
      screen.getByRole("button", { name: "Load more" }),
    ).toBeInTheDocument();
  });

  it("hides the load more button when there are no more pages", () => {
    useHistoryStore.setState({ hasMore: false });

    render(<HistoryPanel />);

    expect(
      screen.queryByRole("button", { name: "Load more" }),
    ).not.toBeInTheDocument();
  });

  it("exposes an accessible name on the search input", () => {
    render(<HistoryPanel />);

    expect(
      screen.getByRole("textbox", { name: t("history.search_placeholder") }),
    ).toBeInTheDocument();
  });

  it("calls loadMore when the button is clicked", async () => {
    const user = userEvent.setup();
    // A full first page keeps hasMore true after the mount load() resolves.
    const fullPage = Array.from({ length: 100 }, (_, i) =>
      createMockItem(`h${i}`, `item ${i}`),
    );
    vi.mocked(tauri.getHistory).mockResolvedValueOnce(fullPage);

    render(<HistoryPanel />);

    const loadMoreButton = await screen.findByRole("button", {
      name: "Load more",
    });
    await user.click(loadMoreButton);

    expect(tauri.getHistory).toHaveBeenCalledWith({ limit: 100, offset: 100 });
  });
});

describe("HistoryPanel split editorial reader", () => {
  const itemA = createMockItem("h1", "first raw text", {
    text_formatted: "First formatted text.",
    is_pinned: true,
    mode: "message",
    app_context: "Slack",
  });
  const itemB = createMockItem("h2", "second raw transcript", {
    text_formatted: "Second formatted transcript.",
    is_pinned: false,
    mode: "email",
    app_context: "Mail",
  });

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tauri.getHistory).mockResolvedValue([itemA, itemB]);
    useHistoryStore.setState({
      items: [itemA, itemB],
      loading: false,
      error: null,
      query: "",
      hasMore: false,
    });
  });

  it("displays selected entry in the Editorial Reader and switches Formatted vs Raw STT tabs", async () => {
    const user = userEvent.setup();
    render(<HistoryPanel />);

    expect((await screen.findAllByText("First formatted text.")).length).toBeGreaterThanOrEqual(2);

    await user.click(screen.getByRole("button", { name: "Raw STT" }));
    expect(screen.getByText("first raw text")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Formatted" }));
    expect(screen.getAllByText("First formatted text.").length).toBeGreaterThanOrEqual(2);

    await user.click(screen.getByRole("button", { name: /Second formatted transcript/ }));
    expect(screen.getAllByText("Second formatted transcript.").length).toBeGreaterThanOrEqual(2);
  });

  it("filters between All and Pinned entries", async () => {
    const user = userEvent.setup();
    render(<HistoryPanel />);

    expect(await screen.findByText(/Second formatted transcript/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Pinned" }));
    expect(screen.queryByText(/Second formatted transcript/)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "All" }));
    expect(screen.getByText(/Second formatted transcript/)).toBeInTheDocument();
  });

  it("executes Copy, Re-inject, Pin, and Delete actions from the Editorial Reader", async () => {
    const user = userEvent.setup();
    const writeText = vi
      .spyOn(window.navigator.clipboard, "writeText")
      .mockResolvedValue(undefined);

    render(<HistoryPanel />);
    await screen.findAllByText("First formatted text.");

    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(writeText).toHaveBeenCalledWith("First formatted text.");

    await user.click(screen.getByRole("button", { name: "Re-inject" }));
    expect(tauri.reInject).toHaveBeenCalledWith("h1");

    await user.click(screen.getByRole("button", { name: "Unpin" }));
    expect(tauri.pinHistory).toHaveBeenCalledWith("h1", false);

    await user.click(screen.getByRole("button", { name: "Delete" }));
    expect(tauri.deleteHistory).toHaveBeenCalledWith("h1");
  });

  it("supports Clear unpinned button with confirmation", async () => {
    const user = userEvent.setup();
    render(<HistoryPanel />);
    await screen.findAllByText("First formatted text.");

    await user.click(screen.getByRole("button", { name: "Clear unpinned" }));
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(tauri.clearHistory).toHaveBeenCalledWith(true);
  });
});
