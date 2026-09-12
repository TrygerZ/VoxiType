import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DataDirectoryStep } from "./DataDirectoryStep";
import * as tauri from "../../../lib/tauri";
import { t } from "../../../lib/i18n";

describe("DataDirectoryStep", () => {
  const defaultProps = {
    step: "data_directory" as const,
    currentStepIdx: 4,
    t,
    onBack: vi.fn(),
    onSkip: vi.fn(),
    onContinue: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders active directory without restart button when pending is null", async () => {
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/active/data/dir",
      default: "/default/data/dir",
      pending: null,
      lastError: null,
    });

    render(<DataDirectoryStep {...defaultProps} />);

    expect(await screen.findByText("/active/data/dir")).toBeInTheDocument();
    expect(screen.queryByTestId("pending-data-directory")).not.toBeInTheDocument();
    expect(screen.queryByTestId("restart-app-button")).not.toBeInTheDocument();
  });

  it("renders pending path and restart button when pending is set", async () => {
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/active/data/dir",
      default: "/default/data/dir",
      pending: "/pending/restart/dir",
      lastError: null,
    });

    render(<DataDirectoryStep {...defaultProps} />);

    expect(await screen.findByText("/active/data/dir")).toBeInTheDocument();
    const pendingElement = await screen.findByTestId("pending-data-directory");
    expect(pendingElement).toBeInTheDocument();
    expect(pendingElement).toHaveTextContent("/pending/restart/dir");
    expect(screen.getByTestId("restart-app-button")).toBeInTheDocument();
  });

  it("shows restart button after successful apply", async () => {
    const user = userEvent.setup();
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/active/data/dir",
      default: "/default/data/dir",
      pending: null,
      lastError: null,
    });
    vi.mocked(tauri.pickDataDirectory).mockResolvedValueOnce("/new/chosen/dir");
    vi.mocked(tauri.setDataDirectory).mockResolvedValueOnce(undefined);

    render(<DataDirectoryStep {...defaultProps} />);

    expect(await screen.findByText("/active/data/dir")).toBeInTheDocument();
    expect(screen.queryByTestId("restart-app-button")).not.toBeInTheDocument();

    const chooseBtn = screen.getByRole("button", { name: /Choose folder/i });
    await user.click(chooseBtn);

    const applyBtn = screen.getByRole("button", { name: /Apply/i });
    expect(applyBtn).toBeEnabled();
    await user.click(applyBtn);

    await waitFor(() => {
      expect(tauri.setDataDirectory).toHaveBeenCalledWith("/new/chosen/dir");
    });

    expect(await screen.findByTestId("restart-app-button")).toBeInTheDocument();
    const pendingElement = screen.getByTestId("pending-data-directory");
    expect(pendingElement).toHaveTextContent("/new/chosen/dir");
  });

  it("calls restartApp when restart button is clicked", async () => {
    const user = userEvent.setup();
    vi.mocked(tauri.getDataDirectory).mockResolvedValueOnce({
      active: "/active/data/dir",
      default: "/default/data/dir",
      pending: "/pending/restart/dir",
      lastError: null,
    });
    vi.mocked(tauri.restartApp).mockResolvedValueOnce(undefined);

    render(<DataDirectoryStep {...defaultProps} />);

    const restartBtn = await screen.findByTestId("restart-app-button");
    expect(restartBtn).toBeInTheDocument();
    await user.click(restartBtn);

    expect(tauri.restartApp).toHaveBeenCalledOnce();
  });
});
