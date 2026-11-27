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
      durationSec: 0,
      wordCount: null,
      errorMessage: null,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders Option B compact idle pill when alwaysRender is true", () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).toBeInTheDocument();
    expect(pill).toHaveClass("pointer-events-auto");
    expect(pill).toHaveAttribute("data-animation-phase", "normal");
    expect(pill).toHaveClass("vx-widget-pill--idle");
    expect(screen.getByTestId("widget-dormant-bars")).toBeInTheDocument();
  });

  it("calls resetWidgetIdleTimer on pointerEnter and pointerDown", () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    fireEvent.pointerEnter(pill);
    expect(tauri.resetWidgetIdleTimer).toHaveBeenCalledTimes(1);

    fireEvent.pointerDown(pill);
    expect(tauri.resetWidgetIdleTimer).toHaveBeenCalledTimes(2);
  });

  it("completes Dynamic Island hide sequence: content fade -> capsule morph -> sink out -> ack", async () => {
    vi.useFakeTimers();
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).toHaveAttribute("data-animation-phase", "normal");

    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 101 });
    });

    // Step 1: content fade (100ms) while retaining idle dimensions
    expect(pill).toHaveAttribute("data-animation-phase", "hiding-content");
    expect(pill).toHaveClass("vx-widget-pill--idle");

    // Advance 100ms -> Step 2: shrink into 48x24 capsule
    act(() => {
      vi.advanceTimersByTime(100);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hiding-shrink");
    expect(pill).toHaveClass("vx-widget-pill--capsule");

    // Advance 220ms (160ms shrink + 60ms hold) -> Step 3: sink out
    act(() => {
      vi.advanceTimersByTime(220);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hiding-fade");
    expect(pill).toHaveClass("vx-widget-pill--hidden");

    // Advance 220ms -> Step 4: hidden and ACK sent
    act(() => {
      vi.advanceTimersByTime(220);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hidden");
    expect(tauri.ackWidgetHide).toHaveBeenCalledWith(101);
  });

  it("completes Dynamic Island reveal sequence: capsule rise -> spring bloom -> normal", async () => {
    vi.useFakeTimers();
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");

    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 102 });
    });
    act(() => {
      vi.advanceTimersByTime(540);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hidden");

    await act(async () => {
      eventHandlers["floating_widget_reveal_requested"]?.({ id: 103 });
    });

    // Step 1: rise into capsule
    expect(pill).toHaveAttribute("data-animation-phase", "revealing-fade");
    expect(pill).toHaveClass("vx-widget-pill--capsule");

    // Advance 150ms -> Step 2: bloom to idle pill
    act(() => {
      vi.advanceTimersByTime(150);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "revealing-expand");
    expect(pill).toHaveClass("vx-widget-pill--idle", "vx-widget-pill--bloom");

    // Advance 260ms -> Step 3: normal visible
    act(() => {
      vi.advanceTimersByTime(260);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "normal");
  });

  it("acknowledges reveal requests immediately with the payload id", async () => {
    vi.useFakeTimers();
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");

    await act(async () => {
      eventHandlers["floating_widget_reveal_requested"]?.({ id: 7 });
    });

    expect(tauri.ackWidgetReveal).toHaveBeenCalledWith(7);
    expect(pill).toHaveAttribute("data-animation-phase", "revealing-fade");
  });

  it("ignores reveal requests whose payload fails validation", async () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    const invalidPayloads: unknown[] = [
      { id: "7" },
      { id: Number.NaN },
      {},
      [],
      null,
      "7",
      undefined,
    ];

    for (const payload of invalidPayloads) {
      await act(async () => {
        eventHandlers["floating_widget_reveal_requested"]?.(payload);
      });
    }

    expect(tauri.ackWidgetReveal).not.toHaveBeenCalled();
    expect(pill).toHaveAttribute("data-animation-phase", "normal");
  });

  it("cancels and reverses hide animation when pointer activity arrives during hide", async () => {
    vi.useFakeTimers();
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");

    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 104 });
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hiding-content");

    act(() => {
      vi.advanceTimersByTime(100);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hiding-shrink");

    act(() => {
      fireEvent.pointerEnter(pill);
    });

    expect(pill).toHaveAttribute("data-animation-phase", "normal");
    expect(tauri.resetWidgetIdleTimer).toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(600);
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

      await act(async () => {
        eventHandlers["floating_widget_hide_requested"]?.({ id: 105 });
      });
      expect(pill).toHaveAttribute("data-animation-phase", "hiding-fade");

      act(() => {
        vi.advanceTimersByTime(120);
      });
      expect(pill).toHaveAttribute("data-animation-phase", "hidden");
      expect(tauri.ackWidgetHide).toHaveBeenCalledWith(105);

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

    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 107 });
    });
    act(() => {
      vi.advanceTimersByTime(540);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hidden");

    act(() => {
      useAppStore.setState({ state: "recording" });
    });

    expect(pill).toHaveAttribute("data-animation-phase", "revealing-fade");

    act(() => {
      vi.advanceTimersByTime(150);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "revealing-expand");

    act(() => {
      vi.advanceTimersByTime(260);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "normal");
  });

  it("handles promise rejection gracefully for ackWidgetHide and resetWidgetIdleTimer", async () => {
    vi.useFakeTimers();
    vi.spyOn(tauri, "ackWidgetHide").mockRejectedValue(new Error("IPC failed"));
    vi.spyOn(tauri, "resetWidgetIdleTimer").mockRejectedValue(new Error("IPC failed"));

    render(<FloatingWidget alwaysRender />);
    const pill = screen.getByTestId("floating-widget-pill");

    expect(() => {
      fireEvent.pointerEnter(pill);
    }).not.toThrow();

    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 108 });
    });
    act(() => {
      vi.advanceTimersByTime(540);
    });
    expect(pill).toHaveAttribute("data-animation-phase", "hidden");
  });

  it("shows Saved to History with error border when text missed the target app", () => {
    useAppStore.setState({
      state: "idle",
      wordCount: 4,
      outcome: { injected: false, appContext: "winword" },
    });

    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).not.toHaveClass("border-vx-semantic-success");
    expect(pill).toHaveClass("border-vx-semantic-error", "vx-widget-pill--error");
    expect(screen.getByText("Saved to History")).toBeInTheDocument();
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
    unmount();

    await act(async () => {
      resolveHideListener(hideUnlistenMock);
    });

    expect(hideUnlistenMock).toHaveBeenCalledTimes(1);
  });

  it("renders Option B idle pill with official logo, dormant bars, zero text, and zero glow or glass classes", () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).toHaveClass(
      "bg-vx-bg-elevated",
      "shadow-vx-widget",
      "border-vx-border-default",
      "vx-widget-pill--idle",
    );
    expect(pill.className).not.toMatch(/backdrop-blur|vx-glass|shadow-\[/);

    const logo = screen.getByAltText("VoxiType Logo");
    expect(logo).toHaveAttribute("src", "/logo.png");
    expect(screen.queryByText("Ctrl + Space")).not.toBeInTheDocument();
  });

  it("renders recording state with Ember border, 12-bar Waveform, and timer", () => {
    useAppStore.setState({ state: "recording", durationSec: 8 });
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).toHaveClass(
      "border-vx-accent-ember",
      "vx-widget-pill--recording",
      "vx-widget-pill--bloom",
    );
    expect(screen.getByText("00:08")).toBeInTheDocument();
    expect(pill.querySelectorAll(".vx-wave-bar")).toHaveLength(12);
  });

  it("renders processing and injected success states with Option B dimensions and semantic borders", () => {
    useAppStore.setState({ state: "processing" });
    const { rerender } = render(<FloatingWidget alwaysRender />);

    expect(screen.getByTestId("floating-widget-pill")).toHaveClass(
      "vx-widget-pill--processing",
    );
    expect(screen.getByText("Formatting...")).toBeInTheDocument();

    act(() => {
      useAppStore.setState({
        state: "idle",
        wordCount: 24,
        outcome: { injected: true, appContext: "notepad" },
      });
    });
    rerender(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).toHaveClass("border-vx-semantic-success", "vx-widget-pill--success");
    expect(screen.getByText("24 words")).toBeInTheDocument();
  });

  it("renders capsule phase with border-vx-border-strong and visible centered logo", async () => {
    vi.useFakeTimers();
    render(<FloatingWidget alwaysRender />);

    await act(async () => {
      eventHandlers["floating_widget_hide_requested"]?.({ id: 201 });
    });
    act(() => {
      vi.advanceTimersByTime(100);
    });

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).toHaveClass("vx-widget-pill--capsule", "border-vx-border-strong");
    const logoWrapper = screen.getByAltText("VoxiType Logo").parentElement;
    expect(logoWrapper).not.toHaveClass("vx-widget-content--hidden");
  });
});
