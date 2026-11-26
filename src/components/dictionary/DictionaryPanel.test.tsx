import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DictionaryPanel } from "./DictionaryPanel";
import * as tauri from "../../lib/tauri";
import { useDictionaryStore } from "../../stores/dictionaryStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { DictionaryEntry } from "../../types/app";

const SAMPLE_ENTRIES: DictionaryEntry[] = [
  {
    id: "d1",
    word: "voxy type",
    pronunciation: null,
    category: "custom",
    replacement: "VoxiType",
    language: "en",
    usage_count: 48,
    is_active: true,
  },
  {
    id: "d2",
    word: "kubernetes",
    pronunciation: null,
    category: "custom",
    replacement: null,
    language: "en",
    usage_count: 12,
    is_active: true,
  },
  {
    id: "d3",
    word: "kalo misalkan",
    pronunciation: null,
    category: "custom",
    replacement: "kalau misalnya",
    language: "id",
    usage_count: 5,
    is_active: false,
  },
];

describe("DictionaryPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tauri.getDictionary).mockResolvedValue(SAMPLE_ENTRIES);
    useSettingsStore.setState({ settings: { language: "en", stt_language: "en" } });
    useDictionaryStore.setState({
      entries: SAMPLE_ENTRIES,
      loading: false,
      error: null,
    });
  });

  it("disables Add when word is blank and submits trimmed entry with language", async () => {
    const user = userEvent.setup();
    render(<DictionaryPanel />);

    const addBtn = screen.getByRole("button", { name: "+ Add" });
    expect(addBtn).toBeDisabled();

    await user.type(
      screen.getByPlaceholderText("Word or spoken phrase (e.g. voxy type)"),
      "tauri",
    );
    await user.type(
      screen.getByPlaceholderText("Replacement (optional, e.g. VoxiType)"),
      "Tauri",
    );
    expect(addBtn).toBeEnabled();

    await user.click(addBtn);

    await waitFor(() => {
      expect(tauri.addDictionaryWord).toHaveBeenCalledWith(
        expect.objectContaining({
          word: "tauri",
          replacement: "Tauri",
          language: "en",
          is_active: true,
        }),
      );
    });
  });

  it("filters entries by search query and All / Active / Replacements chips", async () => {
    const user = userEvent.setup();
    render(<DictionaryPanel />);

    await screen.findByText("voxy type");

    await user.click(screen.getByRole("button", { name: "Active (2)" }));
    expect(screen.queryByText("kalo misalkan")).not.toBeInTheDocument();
    expect(screen.getByText("kubernetes")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Replacements (2)" }));
    expect(screen.queryByText("kubernetes")).not.toBeInTheDocument();
    expect(screen.getByText("voxy type")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "All (3)" }));
    await user.type(
      screen.getByPlaceholderText("Filter 3 dictionary entries..."),
      "kube",
    );
    expect(screen.getByText("kubernetes")).toBeInTheDocument();
    expect(screen.queryByText("voxy type")).not.toBeInTheDocument();
  });

  it("supports inline edit preserving entry id, active Switch toggle, and delete", async () => {
    const user = userEvent.setup();
    render(<DictionaryPanel />);
    await screen.findByText("voxy type");

    await user.click(screen.getByRole("button", { name: "Edit voxy type" }));
    const repInput = screen.getByDisplayValue("VoxiType");
    await user.clear(repInput);
    await user.type(repInput, "VoxiType Pro");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(tauri.addDictionaryWord).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "d1",
          word: "voxy type",
          replacement: "VoxiType Pro",
        }),
      );
    });

    await user.click(
      screen.getByRole("switch", { name: "Deactivate voxy type" }),
    );
    expect(tauri.setDictionaryActive).toHaveBeenCalledWith("d1", false);

    await user.click(screen.getByRole("button", { name: "Delete kubernetes" }));
    expect(tauri.deleteDictionaryWord).toHaveBeenCalledWith("d2");
  });
});
