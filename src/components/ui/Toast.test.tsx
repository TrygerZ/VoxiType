import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ToastContainer, toast } from "./Toast";
import { useToastStore } from "../../stores/toastStore";

describe("ToastContainer", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useToastStore.setState({ toasts: [] });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("keeps an error toast past 3 s and removes it after 8 s", () => {
    render(<ToastContainer />);

    act(() => {
      toast("something failed", "error");
    });
    expect(screen.getByRole("alert")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(3000);
    });
    expect(screen.getByRole("alert")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(5000);
    });
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("keeps an actionable toast until the action runs", () => {
    const onAction = vi.fn();
    render(<ToastContainer />);

    act(() => {
      toast("insert failed", "error", { label: "Open Settings", onClick: onAction });
    });
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.getByRole("alert")).toBeInTheDocument();

    act(() => {
      screen.getByRole("button", { name: "Open Settings" }).click();
    });
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("announces the toast type for non-error toasts", () => {
    render(<ToastContainer />);

    act(() => {
      toast("saved", "success");
    });

    expect(screen.getByText("Success:")).toHaveClass("sr-only");
  });

  it("removes a success toast after 3 s", () => {
    render(<ToastContainer />);

    act(() => {
      toast("saved", "success");
    });
    expect(screen.getByText("saved")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(3000);
    });
    expect(screen.queryByText("saved")).toBeNull();
  });

  it("replaces the collapsed toast action with the newest one", () => {
    const stale = vi.fn();
    const latest = vi.fn();
    render(<ToastContainer />);

    act(() => {
      toast("insert failed", "error", { label: "Copy", onClick: stale });
      toast("insert failed", "error", { label: "Copy", onClick: latest });
    });
    expect(useToastStore.getState().toasts).toHaveLength(1);

    act(() => {
      screen.getByRole("button", { name: "Copy" }).click();
    });
    expect(stale).not.toHaveBeenCalled();
    expect(latest).toHaveBeenCalledTimes(1);
  });

  it("keeps the existing action when a duplicate arrives without one", () => {
    const onAction = vi.fn();
    render(<ToastContainer />);

    act(() => {
      toast("insert failed", "error", { label: "Copy", onClick: onAction });
      toast("insert failed", "error");
    });
    expect(screen.getByRole("button", { name: "Copy" })).toBeInTheDocument();
  });

  it("positions container at top-4 right-4 and renders matte card without left stripe or translucent fill", () => {
    const { container } = render(<ToastContainer />);

    act(() => {
      toast(
        "Restart required",
        "info",
        { label: "Restart", onClick: vi.fn() },
        "Data directory changes apply after restart.",
      );
    });

    const wrapper = container.firstElementChild as HTMLElement;
    expect(wrapper.className).toContain("top-4");
    expect(wrapper.className).toContain("right-4");
    expect(wrapper.className).not.toContain("bottom-4");

    expect(
      screen.getByText("Data directory changes apply after restart."),
    ).toBeInTheDocument();

    const card = screen.getByText("Restart required").closest("[data-testid='toast-card']") as HTMLElement;
    expect(card).not.toBeNull();
    expect(card.className).toContain("bg-vx-bg-elevated");
    expect(card.className).toContain("border-vx-border-default");
    expect(card.className).not.toContain("/10");
    expect(card.className).not.toContain("border-l-");
  });

  it("renders a dismiss button across Info, Success, and Error toasts even with action", () => {
    render(<ToastContainer />);

    act(() => {
      toast("Info notice", "info", { label: "Act", onClick: vi.fn() });
      toast("Success notice", "success");
      toast("Error notice", "error", { label: "Fix", onClick: vi.fn() });
    });

    const dismissButtons = screen.getAllByRole("button", { name: "Dismiss" });
    expect(dismissButtons).toHaveLength(3);

    act(() => {
      dismissButtons[0].click();
    });
    expect(screen.queryByText("Info notice")).toBeNull();
    expect(screen.getByText("Success notice")).toBeInTheDocument();
    expect(screen.getByText("Error notice")).toBeInTheDocument();
  });
});
