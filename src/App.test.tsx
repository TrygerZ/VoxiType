import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { useSettingsStore } from "./stores/settingsStore";
import { useToastStore, toast } from "./stores/toastStore";
import * as tauri from "./lib/tauri";

describe("App onboarding gating", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({
      settings: {},
      loaded: false,
      error: null,
    });
    useToastStore.setState({ toasts: [] });
  });

  it("does not render onboarding when settings load fails with error", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.mocked(tauri.getSettings).mockRejectedValueOnce(
      new Error("IPC communication failed"),
    );

    render(<App />);

    // Wait for store to complete load with error. The raw IPC message is
    // tucked behind "Technical details" instead of being the headline.
    expect(await screen.findByText("VoxiType could not start")).toBeInTheDocument();
    expect(
      screen.getByText("Technical details"),
    ).toBeInTheDocument();
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
    expect(screen.queryByText("VoxiType could not start")).not.toBeInTheDocument();
    consoleSpy.mockRestore();
  });

  it("renders toasts raised during onboarding", async () => {
    vi.mocked(tauri.getSettings).mockResolvedValue({});

    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Welcome to VoxiType" }),
    ).toBeInTheDocument();

    act(() => {
      toast("hotkey unavailable", "info");
    });

    expect(screen.getByText("hotkey unavailable")).toBeInTheDocument();
  });
});
