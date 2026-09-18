import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { STTTab } from "./STTTab";
import * as tauri from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";

describe("STTTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls testGroqApi with null when groqKeySet is true and input field is empty", async () => {
    const user = userEvent.setup();
    useSettingsStore.setState({
      settings: {
        language: "id",
        stt_engine: "groq",
        groq_api_key: "",
        groq_api_key_set: true,
      },
      loaded: true,
      error: null,
    });

    render(<STTTab />);

    const testButton = screen.getByRole("button", { name: "Tes Koneksi" });
    expect(testButton).toBeEnabled();

    await user.click(testButton);

    await waitFor(() => {
      expect(tauri.testGroqApi).toHaveBeenCalledWith(null);
    });
    expect(tauri.testGroqApi).not.toHaveBeenCalledWith("");
  });

  it("calls testGroqApi with explicit key when user inputs a key", async () => {
    const user = userEvent.setup();
    useSettingsStore.setState({
      settings: {
        language: "id",
        stt_engine: "groq",
        groq_api_key: "",
        groq_api_key_set: true,
      },
      loaded: true,
      error: null,
    });

    render(<STTTab />);

    const keyInput = screen.getByPlaceholderText("Tersimpan");
    await user.clear(keyInput);
    await user.type(keyInput, "gsk_new_secret_key");

    const testButton = screen.getByRole("button", { name: "Tes Koneksi" });
    await user.click(testButton);

    await waitFor(() => {
      expect(tauri.testGroqApi).toHaveBeenCalledWith("gsk_new_secret_key");
    });
  });

  it("disables test connection button when field is empty and groqKeySet is false", () => {
    useSettingsStore.setState({
      settings: {
        language: "id",
        stt_engine: "groq",
        groq_api_key: "",
        groq_api_key_set: false,
      },
      loaded: true,
      error: null,
    });

    render(<STTTab />);

    const testButton = screen.getByRole("button", { name: "Tes Koneksi" });
    expect(testButton).toBeDisabled();
  });
});
