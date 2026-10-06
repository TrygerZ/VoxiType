import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { debounce } from "./debounce";

describe("debounce", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("calls once after the delay with the latest args", () => {
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced("first");
    debounced("second");

    expect(fn).not.toHaveBeenCalled();
    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith("second");
  });

  it("flush invokes immediately with the latest args and cancels the timer", () => {
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced("first");
    debounced("latest");
    debounced.flush();

    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith("latest");

    // The pending timer must not fire a second time.
    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("flush with nothing pending is a no-op", () => {
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced.flush();

    expect(fn).not.toHaveBeenCalled();
  });

  it("cancel discards a pending call so a later timer run does not fire", () => {
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced("value");
    debounced.cancel();

    vi.advanceTimersByTime(100);
    expect(fn).not.toHaveBeenCalled();
  });

  it("flush after cancel is a no-op", () => {
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced("value");
    debounced.cancel();
    debounced.flush();

    expect(fn).not.toHaveBeenCalled();
  });
});
