import { beforeEach, describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { FileTranscriptionPanel } from "./FileTranscriptionPanel";
import { useFileTranscriptionStore } from "../../stores/fileTranscriptionStore";
import { useSettingsStore } from "../../stores/settingsStore";

describe("FileTranscriptionPanel", () => {
  beforeEach(() => {
    useSettingsStore.setState({ settings: { groq_api_key_set: true, llm_engine: "groq" } });
    useFileTranscriptionStore.setState({
      path: "C:\\audio\\meeting.mp3",
      options: null,
      picking: false,
      status: "idle",
      progress: null,
      result: null,
      error: null,
      cancelled: false,
    });
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
});
