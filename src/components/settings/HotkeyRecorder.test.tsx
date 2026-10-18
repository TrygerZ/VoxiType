import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { formatAccelerator, HotkeyRecorder } from "./HotkeyRecorder";

function dispatchKey(init: KeyboardEventInit): KeyboardEvent {
  const event = new KeyboardEvent("keydown", {
    cancelable: true,
    bubbles: true,
    ...init,
  });
  act(() => {
    window.dispatchEvent(event);
  });
  return event;
}

async function startRecording() {
  const user = userEvent.setup();
  await user.click(screen.getByRole("button"));
  expect(screen.getByText("Press your shortcut...")).toBeInTheDocument();
}

describe("formatAccelerator", () => {
  it("renders Super as Cmd on macOS", () => {
    expect(formatAccelerator("Super+Shift+Space", "macos")).toBe(
      "Cmd+Shift+Space",
    );
  });

  it("keeps Super on other platforms", () => {
    expect(formatAccelerator("Super+Shift+Space", "linux")).toBe(
      "Super+Shift+Space",
    );
  });
});

describe("HotkeyRecorder", () => {
  it("does not prevent default for Tab so focus can move", async () => {
    render(<HotkeyRecorder value="Ctrl+Space" onChange={vi.fn()} />);
    await startRecording();

    const event = dispatchKey({ key: "Tab" });

    expect(event.defaultPrevented).toBe(false);
  });

  it("does not consume a plain key without modifiers and keeps recording", async () => {
    const onChange = vi.fn();
    render(<HotkeyRecorder value="Ctrl+Space" onChange={onChange} />);
    await startRecording();

    const event = dispatchKey({ key: "a" });

    expect(event.defaultPrevented).toBe(false);
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByText("Press your shortcut...")).toBeInTheDocument();
  });

  it("accepts a modifier combo, calls onChange, and prevents default", async () => {
    const onChange = vi.fn();
    render(<HotkeyRecorder value="Ctrl+Space" onChange={onChange} />);
    await startRecording();

    const event = dispatchKey({ key: "k", ctrlKey: true });

    expect(onChange).toHaveBeenCalledWith("Ctrl+K");
    expect(event.defaultPrevented).toBe(true);
    expect(screen.queryByText("Press your shortcut...")).toBeNull();
  });

  it("cancels recording on Escape", async () => {
    render(<HotkeyRecorder value="Ctrl+Space" onChange={vi.fn()} />);
    await startRecording();

    const event = dispatchKey({ key: "Escape" });

    expect(event.defaultPrevented).toBe(true);
    expect(screen.queryByText("Press your shortcut...")).toBeNull();
    expect(screen.getByText("Click to record a new shortcut")).toBeInTheDocument();
  });
});
