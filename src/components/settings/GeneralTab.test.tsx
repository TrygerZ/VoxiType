import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { GeneralTab } from "./GeneralTab";
import * as tauri from "../../lib/tauri";
import { useSettingsStore } from "../../stores/settingsStore";

describe("GeneralTab data directory settings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({
      settings: {
        language: "id",
        floating_widget: true,
      },
      loaded: true,
      error: null,
    });
  });

  it("renders normal state with active directory and no pending or last_error", async () => {
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/active/data/dir",
      default: "/default/data/dir",
      pending: null,
      lastError: null,
    });

    render(<GeneralTab />);

    expect(await screen.findByTestId("active-data-directory")).toHaveTextContent(
      "/active/data/dir",
    );
    expect(screen.queryByTestId("pending-data-directory")).not.toBeInTheDocument();
    expect(screen.queryByTestId("restart-app-button")).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("data-directory-last-error"),
    ).not.toBeInTheDocument();
  });

  it("renders pending state when directory is awaiting restart", async () => {
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/active/data/dir",
      default: "/default/data/dir",
      pending: "/pending/restart/dir",
      lastError: null,
    });

    render(<GeneralTab />);

    expect(await screen.findByTestId("active-data-directory")).toHaveTextContent(
      "/active/data/dir",
    );
    const pendingElement = await screen.findByTestId("pending-data-directory");
    expect(pendingElement).toBeInTheDocument();
    expect(pendingElement).toHaveTextContent("/pending/restart/dir");
    expect(screen.getByTestId("restart-app-button")).toBeInTheDocument();
    expect(
      screen.queryByTestId("data-directory-last-error"),
    ).not.toBeInTheDocument();
  });

  it("renders last_error when previous directory migration failed", async () => {
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/default/data/dir",
      default: "/default/data/dir",
      pending: null,
      lastError: "Target directory not writable",
    });

    render(<GeneralTab />);

    expect(await screen.findByTestId("active-data-directory")).toHaveTextContent(
      "/default/data/dir",
    );
    const errorElement = await screen.findByTestId("data-directory-last-error");
    expect(errorElement).toBeInTheDocument();
    expect(errorElement).toHaveTextContent("Target directory not writable");
  });

  it("successful Apply sets pending and does not overwrite active location", async () => {
    const user = userEvent.setup();
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/active/data/dir",
      default: "/default/data/dir",
      pending: null,
      lastError: null,
    });
    vi.mocked(tauri.pickDataDirectory).mockResolvedValueOnce("/new/chosen/dir");
    vi.mocked(tauri.setDataDirectory).mockResolvedValueOnce(undefined);

    render(<GeneralTab />);

    expect(await screen.findByTestId("active-data-directory")).toHaveTextContent(
      "/active/data/dir",
    );
    const applyButton = screen.getByRole("button", { name: "Terapkan" });
    expect(applyButton).toBeDisabled();

    // Click Choose folder
    const chooseButton = screen.getByRole("button", { name: /Pilih folder/i });
    await user.click(chooseButton);

    // Selected path is shown and Apply button is enabled
    expect(
      await screen.findByTestId("selected-data-directory"),
    ).toHaveTextContent("/new/chosen/dir");
    expect(applyButton).toBeEnabled();

    // Click Apply
    await user.click(applyButton);

    await waitFor(() => {
      expect(tauri.setDataDirectory).toHaveBeenCalledWith("/new/chosen/dir");
    });

    // Assert active location label is STILL unchanged
    expect(screen.getByTestId("active-data-directory")).toHaveTextContent(
      "/active/data/dir",
    );

    // Assert pending restart element is shown with the new path
    const pendingElement = await screen.findByTestId("pending-data-directory");
    expect(pendingElement).toHaveTextContent("/new/chosen/dir");
    expect(screen.getByTestId("restart-app-button")).toBeInTheDocument();

    // Assert success status is displayed
    expect(screen.getByTestId("data-directory-status")).toBeInTheDocument();
    // Selected preview is cleared
    expect(screen.queryByTestId("selected-data-directory")).not.toBeInTheDocument();
  });

  it("renders loading state while data directory is fetching", () => {
    vi.mocked(tauri.getDataDirectory).mockReturnValueOnce(new Promise(() => {}));

    render(<GeneralTab />);

    expect(screen.getByTestId("active-data-directory")).toHaveTextContent(
      "Memuat lokasi...",
    );
  });

  it("announces status and error with proper accessibility roles", async () => {
    const user = userEvent.setup();
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/default/data/dir",
      default: "/default/data/dir",
      pending: null,
      lastError: "Previous migration failed",
    });
    vi.mocked(tauri.pickDataDirectory).mockResolvedValueOnce("/new/path");
    vi.mocked(tauri.setDataDirectory).mockResolvedValueOnce(undefined);

    render(<GeneralTab />);

    const lastErrorEl = await screen.findByTestId("data-directory-last-error");
    expect(lastErrorEl).toHaveAttribute("role", "alert");

    const chooseButton = screen.getByRole("button", { name: /Pilih folder/i });
    await user.click(chooseButton);
    const applyButton = screen.getByRole("button", { name: "Terapkan" });
    await user.click(applyButton);

    const statusEl = await screen.findByTestId("data-directory-status");
    expect(statusEl).toHaveAttribute("role", "status");
  });

  it("disables choose and apply buttons while operation is in-flight", async () => {
    let resolvePick: ((path: string) => void) | undefined;
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/active/data/dir",
      default: "/default/data/dir",
      pending: null,
      lastError: null,
    });
    vi.mocked(tauri.pickDataDirectory).mockReturnValueOnce(
      new Promise((resolve) => {
        resolvePick = resolve;
      }),
    );

    const user = userEvent.setup();
    render(<GeneralTab />);

    await screen.findByTestId("active-data-directory");
    const chooseButton = screen.getByRole("button", { name: /Pilih folder/i });
    const applyButton = screen.getByRole("button", { name: "Terapkan" });

    expect(chooseButton).toBeEnabled();
    expect(applyButton).toBeDisabled();

    const clickPromise = user.click(chooseButton);
    await waitFor(() => {
      expect(chooseButton).toBeDisabled();
    });

    resolvePick!("/resolved/path");
    await clickPromise;

    await waitFor(() => {
      expect(chooseButton).toBeEnabled();
      expect(applyButton).toBeEnabled();
    });
  });

  it("clicking restart button calls restartApp", async () => {
    const user = userEvent.setup();
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/active/data/dir",
      default: "/default/data/dir",
      pending: "/pending/restart/dir",
      lastError: null,
    });
    vi.mocked(tauri.restartApp).mockResolvedValueOnce(undefined);

    render(<GeneralTab />);

    const restartBtn = await screen.findByTestId("restart-app-button");
    expect(restartBtn).toBeInTheDocument();
    await user.click(restartBtn);

    expect(tauri.restartApp).toHaveBeenCalledOnce();
  });
});

describe("GeneralTab auto-hide floating widget settings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({
      settings: {
        language: "id",
        floating_widget: true,
        floating_widget_auto_hide_seconds: 0,
      },
      loaded: true,
      error: null,
    });
  });

  it("renders toggle switch and number input with initial disabled state when off", () => {
    render(<GeneralTab />);

    const toggle = screen.getByTestId("widget-auto-hide-switch");
    const input = screen.getByTestId("widget-auto-hide-input");

    expect(toggle).toBeInTheDocument();
    expect(toggle).toHaveAttribute("aria-checked", "false");
    expect(input).toBeInTheDocument();
    expect(input).toBeDisabled();
    expect(input).toHaveValue(3);
  });

  it("toggling auto-hide ON persists minimum default (3 seconds)", async () => {
    const user = userEvent.setup();
    render(<GeneralTab />);

    const toggle = screen.getByTestId("widget-auto-hide-switch");
    await user.click(toggle);

    await waitFor(() => {
      expect(tauri.updateSetting).toHaveBeenCalledWith(
        "floating_widget_auto_hide_seconds",
        3,
      );
    });
  });

  it("toggling auto-hide OFF persists 0 (disabled)", async () => {
    useSettingsStore.setState({
      settings: {
        language: "id",
        floating_widget: true,
        floating_widget_auto_hide_seconds: 10,
      },
      loaded: true,
      error: null,
    });

    const user = userEvent.setup();
    render(<GeneralTab />);

    const toggle = screen.getByTestId("widget-auto-hide-switch");
    expect(toggle).toHaveAttribute("aria-checked", "true");
    const input = screen.getByTestId("widget-auto-hide-input");
    expect(input).toBeEnabled();
    expect(input).toHaveValue(10);

    await user.click(toggle);

    await waitFor(() => {
      expect(tauri.updateSetting).toHaveBeenCalledWith(
        "floating_widget_auto_hide_seconds",
        0,
      );
    });
  });

  it("valid number change within 3-60 persists new value", async () => {
    useSettingsStore.setState({
      settings: {
        language: "id",
        floating_widget: true,
        floating_widget_auto_hide_seconds: 5,
      },
      loaded: true,
      error: null,
    });

    const user = userEvent.setup();
    render(<GeneralTab />);

    const input = screen.getByTestId("widget-auto-hide-input");
    await user.clear(input);
    await user.type(input, "15");

    await waitFor(() => {
      expect(tauri.updateSetting).toHaveBeenCalledWith(
        "floating_widget_auto_hide_seconds",
        15,
      );
    });
  });

  it("invalid number changes outside 3-60 are rejected/ignored by UI", async () => {
    useSettingsStore.setState({
      settings: {
        language: "id",
        floating_widget: true,
        floating_widget_auto_hide_seconds: 5,
      },
      loaded: true,
      error: null,
    });

    const user = userEvent.setup();
    render(<GeneralTab />);

    const input = screen.getByTestId("widget-auto-hide-input");
    vi.mocked(tauri.updateSetting).mockClear();

    // Value below minimum (2 < 3)
    await user.clear(input);
    await user.type(input, "2");
    expect(tauri.updateSetting).not.toHaveBeenCalled();

    // Value above maximum (100 > 60)
    fireEvent.change(input, { target: { value: "100" } });
    expect(tauri.updateSetting).not.toHaveBeenCalled();

    // Negative value (-1 < 3)
    fireEvent.change(input, { target: { value: "-1" } });
    expect(tauri.updateSetting).not.toHaveBeenCalled();
  });

  it("disables auto-hide controls when floating widget is disabled", () => {
    useSettingsStore.setState({
      settings: {
        language: "id",
        floating_widget: false,
        floating_widget_auto_hide_seconds: 5,
      },
      loaded: true,
      error: null,
    });

    render(<GeneralTab />);

    const toggle = screen.getByTestId("widget-auto-hide-switch");
    const input = screen.getByTestId("widget-auto-hide-input");

    expect(toggle).toBeDisabled();
    expect(input).toBeDisabled();
  });
});
