import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { LLMTab } from "./LLMTab";
import * as tauri from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";

describe("LLMTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls testGroqApi with null when groqKeySet is true and input field is empty", async () => {
    const user = userEvent.setup();
    useSettingsStore.setState({
      settings: {
        language: "en",
        llm_engine: "groq",
        groq_api_key: "",
        groq_api_key_set: true,
      },
      loaded: true,
      error: null,
    });

    const { container } = render(<LLMTab />);
    expect(container.querySelector(".text-green-600")).toBeNull();

    const btn = screen.getByRole("button", { name: "Test Connection" });
    expect(btn).toBeEnabled();
    await user.click(btn);

    await waitFor(() => {
      expect(tauri.testGroqApi).toHaveBeenCalledWith(null);
    });
    expect(tauri.testGroqApi).not.toHaveBeenCalledWith("");
  });

  it("tests Ollama readiness with the configured model and displays ready badge", async () => {
    const user = userEvent.setup();
    useSettingsStore.setState({
      settings: {
        language: "en",
        llm_engine: "ollama",
        llm_model: "qwen2.5:3b",
      },
      loaded: true,
      error: null,
    });

    render(<LLMTab />);

    await user.click(screen.getByRole("button", { name: "Test Ollama" }));

    await waitFor(() => {
      expect(tauri.testOllama).toHaveBeenCalledWith("qwen2.5:3b");
    });
    expect(await screen.findByText("qwen2.5:3b ready")).toBeInTheDocument();
  });
});
