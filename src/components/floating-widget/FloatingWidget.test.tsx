import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FloatingWidget } from "./FloatingWidget";
import * as tauri from "../../lib/tauri";
import { useAppStore } from "../../stores/appStore";

describe("FloatingWidget", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      state: "idle",
      audioLevel: 0,
      durationSec: 0,
      wordCount: null,
      errorMessage: null,
    });
  });

  it("renders when alwaysRender is true in idle state", () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    expect(pill).toBeInTheDocument();
    expect(pill).toHaveClass("pointer-events-auto");
  });

  it("calls resetWidgetIdleTimer on pointerEnter", () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    fireEvent.pointerEnter(pill);

    expect(tauri.resetWidgetIdleTimer).toHaveBeenCalledTimes(1);
  });

  it("calls resetWidgetIdleTimer on pointerDown", () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    fireEvent.pointerDown(pill);

    expect(tauri.resetWidgetIdleTimer).toHaveBeenCalledTimes(1);
  });

  it("resets idle timer on repeated pointer interactions", () => {
    render(<FloatingWidget alwaysRender />);

    const pill = screen.getByTestId("floating-widget-pill");
    fireEvent.pointerEnter(pill);
    fireEvent.pointerDown(pill);
    fireEvent.pointerDown(pill);

    expect(tauri.resetWidgetIdleTimer).toHaveBeenCalledTimes(3);
  });
});
