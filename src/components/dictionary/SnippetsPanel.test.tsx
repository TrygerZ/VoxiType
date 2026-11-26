import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SnippetsPanel } from "./SnippetsPanel";
import * as tauri from "../../lib/tauri";
import { useSnippetStore } from "../../stores/snippetStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { Snippet } from "../../types/app";

const SAMPLE_SNIPPETS: Snippet[] = [
  {
    id: "s1",
    name: "my calendar link",
    trigger_phrase: "my calendar link",
    content: "https://cal.com/voxitype/30min",
    category: null,
    mode: null,
    usage_count: 27,
    is_active: true,
  },
  {
    id: "s2",
    name: "standup template",
    trigger_phrase: "standup template",
    content: "Yesterday: IPC audit. Today: Design system.",
    category: null,
    mode: null,
    usage_count: 9,
    is_active: false,
  },
];

describe("SnippetsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tauri.getSnippets).mockResolvedValue(SAMPLE_SNIPPETS);
    useSettingsStore.setState({ settings: { language: "en" } });
    useSnippetStore.setState({
      snippets: SAMPLE_SNIPPETS,
      loading: false,
      error: null,
    });
  });

  it("shows active count badge, prevents Enter submit without content, and disables Add until both fields are set", async () => {
    const user = userEvent.setup();
    render(<SnippetsPanel />);

    expect(await screen.findByText("1 active triggers")).toBeInTheDocument();

    const addBtn = screen.getByRole("button", { name: "+ Add snippet" });
    expect(addBtn).toBeDisabled();

    const triggerInput = screen.getByPlaceholderText(
      "Trigger phrase (e.g. insert signoff)",
    );
    await user.type(triggerInput, "formal signoff{Enter}");
    expect(addBtn).toBeDisabled();
    expect(tauri.addSnippet).not.toHaveBeenCalled();

    await user.type(
      screen.getByPlaceholderText("Expanded text content..."),
      "Best regards, VoxiType",
    );
    expect(addBtn).toBeEnabled();

    await user.click(addBtn);
    await waitFor(() => {
      expect(tauri.addSnippet).toHaveBeenCalledWith(
        expect.objectContaining({
          trigger_phrase: "formal signoff",
          content: "Best regards, VoxiType",
          is_active: true,
        }),
      );
    });
  });

  it("filters snippets by trigger or content and supports Switch toggle, inline edit, and delete", async () => {
    const user = userEvent.setup();
    render(<SnippetsPanel />);
    await screen.findByText("my calendar link");

    await user.type(
      screen.getByPlaceholderText("Search snippets by trigger or content..."),
      "Design system",
    );
    expect(screen.queryByText("my calendar link")).not.toBeInTheDocument();
    expect(screen.getByText("standup template")).toBeInTheDocument();

    await user.clear(
      screen.getByPlaceholderText("Search snippets by trigger or content..."),
    );

    await user.click(
      screen.getByRole("switch", { name: "Deactivate my calendar link" }),
    );
    expect(tauri.setSnippetActive).toHaveBeenCalledWith("s1", false);

    await user.click(
      screen.getByRole("button", { name: "Edit standup template" }),
    );
    const contentBox = screen.getByDisplayValue(
      "Yesterday: IPC audit. Today: Design system.",
    );
    await user.clear(contentBox);
    await user.type(contentBox, "Updated standup notes");
    await user.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(tauri.addSnippet).toHaveBeenCalledWith(
        expect.objectContaining({
          id: "s2",
          trigger_phrase: "standup template",
          content: "Updated standup notes",
        }),
      );
    });

    await user.click(
      screen.getByRole("button", { name: "Delete standup template" }),
    );
    expect(tauri.deleteSnippet).toHaveBeenCalledWith("s2");
  });
});
