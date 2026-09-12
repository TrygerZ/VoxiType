import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { useSettingsStore } from "./stores/settingsStore";
import * as tauri from "./lib/tauri";

describe("App onboarding gating", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({
      settings: {},
      loaded: false,
      error: null,
    });
  });

  it("does not render onboarding when settings load fails with error", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.mocked(tauri.getSettings).mockRejectedValueOnce(
      new Error("IPC communication failed"),
    );

    render(<App />);

    // Wait for store to complete load with error
    expect(
      await screen.findByText("Failed to load settings"),
    ).toBeInTheDocument();
    expect(screen.getByText("IPC communication failed")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Retry" }),
    ).toBeInTheDocument();

    // Verify onboarding is NOT rendered
    expect(
      screen.queryByRole("heading", { name: "Welcome to VoxiType" }),
    ).not.toBeInTheDocument();
    consoleSpy.mockRestore();
  });

  it("does not render onboarding when onboarding_completed is true", async () => {
    vi.mocked(tauri.getSettings).mockResolvedValue({
      onboarding_completed: true,
    });

    render(<App />);

    // Wait for main screen (floating dock home navigation button)
    expect(
      await screen.findByRole("button", { name: "Home" }),
    ).toBeInTheDocument();

    // Verify onboarding is NOT rendered
    expect(
      screen.queryByRole("heading", { name: "Welcome to VoxiType" }),
    ).not.toBeInTheDocument();
  });

  it("renders onboarding when loaded without error and onboarding_completed is absent", async () => {
    vi.mocked(tauri.getSettings).mockResolvedValue({});

    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Welcome to VoxiType" }),
    ).toBeInTheDocument();
  });

  it("allows retrying when settings load fails", async () => {
    const user = userEvent.setup();
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.mocked(tauri.getSettings).mockRejectedValueOnce(
      new Error("initial failure"),
    );

    render(<App />);

    const retryBtn = await screen.findByRole("button", { name: "Retry" });
    expect(retryBtn).toBeInTheDocument();

    vi.mocked(tauri.getSettings).mockResolvedValue({
      onboarding_completed: true,
    });

    await user.click(retryBtn);

    expect(
      await screen.findByRole("button", { name: "Home" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Failed to load settings")).not.toBeInTheDocument();
    consoleSpy.mockRestore();
  });
});
