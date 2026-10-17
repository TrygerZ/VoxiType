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
});
