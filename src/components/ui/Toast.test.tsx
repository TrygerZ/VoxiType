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
});
