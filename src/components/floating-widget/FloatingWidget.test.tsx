import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FloatingWidget } from "./FloatingWidget";
import * as tauri from "../../lib/tauri";
import { useAppStore } from "../../stores/appStore";

describe("FloatingWidget", () => {
  let eventHandlers: Record<string, (payload: unknown) => void> = {};

  beforeEach(() => {
    vi.clearAllMocks();
    eventHandlers = {};

    vi.spyOn(tauri, "onEvent").mockImplementation(
      (event: string, handler: (payload: unknown) => void) => {
        eventHandlers[event] = handler;
        return Promise.resolve(() => {
          delete eventHandlers[event];
        });
      },
    );

    useAppStore.setState({
      state: "idle",
      audioLevel: 0,
      durationSec: 0,
      wordCount: null,
      errorMessage: null,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders when alwaysRender is true in idle state", () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).toBeInTheDocument();
    expect(pill).toHaveClass("pointer-events-auto");
    expect(pill).toHaveAttribute("data-animation-phase", "normal");
    expect(pill).toHaveClass("vx-widget-pill--normal");
  });

  it("calls resetWidgetIdleTimer on pointerEnter and pointerDown", () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    fireEvent.pointerEnter(pill);
    expect(tauri.resetWidgetIdleTimer).toHaveBeenCalledTimes(1);

    fireEvent.pointerDown(pill);
    expect(tauri.resetWidgetIdleTimer).toHaveBeenCalledTimes(2);
  });

  it("completes full hide animation sequence: normal -> capsule -> faded -> ack", async () => {
    vi.useFakeTimers();
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).toHaveAttribute("data-animation-phase", "normal");

    // Backend requests hide
    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 101 });
    });

    // Step 1: shrink into capsule
    expect(pill).toHaveAttribute("data-animation-phase", "hiding-shrink");
    expect(pill).toHaveClass("vx-widget-pill--capsule");

    // Advance 200ms -> Step 2: fade out
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hiding-fade");
    expect(pill).toHaveClass("vx-widget-pill--hidden");

    // Advance 150ms -> Step 3: hidden and ACK sent
    act(() => {
      vi.advanceTimersByTime(150);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hidden");
    expect(tauri.ackWidgetHide).toHaveBeenCalledWith(101);
  });

  it("completes full reveal animation sequence: faded capsule -> visible capsule -> expanded normal", async () => {
    vi.useFakeTimers();
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");

    // Put widget into hidden state through hide sequence
    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 102 });
    });
    act(() => {
      vi.advanceTimersByTime(350);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hidden");

    // Backend reveals window and requests animation
    await act(async () => {
      eventHandlers["floating_widget_reveal_requested"]?.({ id: 103 });
    });

    // Step 1: fade in capsule
    expect(pill).toHaveAttribute("data-animation-phase", "revealing-fade");
    expect(pill).toHaveClass("vx-widget-pill--capsule");

    // Advance 120ms -> Step 2: expand to normal
    act(() => {
      vi.advanceTimersByTime(120);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "revealing-expand");
    expect(pill).toHaveClass("vx-widget-pill--normal");

    // Advance 200ms -> Step 3: normal visible
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "normal");
  });

  it("cancels and reverses hide animation when pointer activity arrives during hide", async () => {
    vi.useFakeTimers();
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");

    // Trigger hide
    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 104 });
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hiding-shrink");

    // Hover occurs during shrink animation
    act(() => {
      fireEvent.pointerEnter(pill);
    });

    // Should immediately revert to normal and reset backend timer
    expect(pill).toHaveAttribute("data-animation-phase", "normal");
    expect(tauri.resetWidgetIdleTimer).toHaveBeenCalled();

    // Advance past hide duration: ackWidgetHide must NOT be called
    act(() => {
      vi.advanceTimersByTime(500);
    });
    expect(tauri.ackWidgetHide).not.toHaveBeenCalled();
  });

  it("respects prefers-reduced-motion by skipping shrink transform during hide and reveal", async () => {
    vi.useFakeTimers();
    const originalMatchMedia = window.matchMedia;
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: query.includes("prefers-reduced-motion"),
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    try {
      render(<FloatingWidget alwaysRender />);
      const pill = screen.getByTestId("floating-widget-pill");

      // Hide request with reduced motion: skips shrink, goes directly to fading
      await act(async () => {
        eventHandlers["floating_widget_hide_requested"]?.({ id: 105 });
      });
      expect(pill).toHaveAttribute("data-animation-phase", "hiding-fade");

      // 120ms fade -> hidden and ACK sent
      act(() => {
        vi.advanceTimersByTime(120);
      });
      expect(pill).toHaveAttribute("data-animation-phase", "hidden");
      expect(tauri.ackWidgetHide).toHaveBeenCalledWith(105);

      // Reveal request with reduced motion
      await act(async () => {
        eventHandlers["floating_widget_reveal_requested"]?.({ id: 106 });
      });
      expect(pill).toHaveAttribute("data-animation-phase", "revealing-fade");

      act(() => {
        vi.advanceTimersByTime(120);
      });
      expect(pill).toHaveAttribute("data-animation-phase", "normal");
    } finally {
      window.matchMedia = originalMatchMedia;
    }
  });

  it("auto-reveals when state changes to recording while hidden", async () => {
    vi.useFakeTimers();
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");

    // Hide widget
    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 107 });
    });
    act(() => {
      vi.advanceTimersByTime(350);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hidden");

    // Recording begins
    act(() => {
      useAppStore.setState({ state: "recording" });
    });

    expect(pill).toHaveAttribute("data-animation-phase", "revealing-fade");

    act(() => {
      vi.advanceTimersByTime(120);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "revealing-expand");

    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "normal");
  });

  it("handles promise rejection gracefully for ackWidgetHide and resetWidgetIdleTimer", async () => {
    vi.useFakeTimers();
    vi.spyOn(tauri, "ackWidgetHide").mockRejectedValue(new Error("IPC failed"));
    vi.spyOn(tauri, "resetWidgetIdleTimer").mockRejectedValue(new Error("IPC failed"));

    render(<FloatingWidget alwaysRender />);
    const pill = screen.getByTestId("floating-widget-pill");

    // Pointer activity rejection should not throw uncaught error
    expect(() => {
      fireEvent.pointerEnter(pill);
    }).not.toThrow();

    // Hide ACK rejection should not throw uncaught error
    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 108 });
    });
    act(() => {
      vi.advanceTimersByTime(350);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hidden");
  });

  it("unregisters listeners immediately if resolved after effect unmount (disposed flag)", async () => {
    let resolveHideListener!: (unlisten: () => void) => void;
    const hideUnlistenMock = vi.fn();

    vi.spyOn(tauri, "onEvent").mockImplementation((event: string) => {
      if (event === "floating_widget_hide_requested") {
        return new Promise((resolve) => {
          resolveHideListener = resolve;
        });
      }
      return Promise.resolve(vi.fn());
    });

    const { unmount } = render(<FloatingWidget alwaysRender />);

    // Unmount before onEvent promise resolves
    unmount();

    // Now async listener promise resolves after unmount
    await act(async () => {
      resolveHideListener(hideUnlistenMock);
    });

    // Disposed check must call unlisten immediately
    expect(hideUnlistenMock).toHaveBeenCalledTimes(1);
  });
});
