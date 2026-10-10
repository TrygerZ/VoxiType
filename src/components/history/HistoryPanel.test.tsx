import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { HistoryPanel } from "./HistoryPanel";
import { useHistoryStore } from "../../stores/historyStore";
import * as tauri from "../../lib/tauri";
import { t } from "../../lib/i18n";
import type { TranscriptionEntry } from "../../types/app";

const createMockItem = (id: string, text: string): TranscriptionEntry => ({
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
