import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useDebouncedApiKey } from "./useDebouncedApiKey";

describe("useDebouncedApiKey", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("does not overwrite user typing when storeValue changes to the value originating from self-persist", async () => {
    const update = vi.fn().mockResolvedValue(undefined);
    const { result, rerender } = renderHook(
      ({ storeValue }) => useDebouncedApiKey(storeValue, update, "Saved"),
      { initialProps: { storeValue: "gsk_initial" } },
    );

    // User types a new key
    act(() => {
      result.current.onKeyChange("gsk_typed_key");
    });
    expect(result.current.localKey).toBe("gsk_typed_key");

    // Fast-forward debounce timer to trigger persist
    act(() => {
      vi.advanceTimersByTime(600);
    });
    expect(update).toHaveBeenCalledWith("groq_api_key", "gsk_typed_key");

    // User continues typing while store updates
    act(() => {
      result.current.onKeyChange("gsk_typed_key_more");
    });
    expect(result.current.localKey).toBe("gsk_typed_key_more");

    // Store syncs back the value that came from the persist itself
    rerender({ storeValue: "gsk_typed_key" });

    // Typing must NOT be wiped or reverted
    expect(result.current.localKey).toBe("gsk_typed_key_more");
  });

  it("resyncs localKey when storeValue changes from an external source", () => {
    const update = vi.fn().mockResolvedValue(undefined);
    const { result, rerender } = renderHook(
      ({ storeValue }) => useDebouncedApiKey(storeValue, update, "Saved"),
      { initialProps: { storeValue: "gsk_initial" } },
    );

    expect(result.current.localKey).toBe("gsk_initial");

    // External reload changes the store value
    rerender({ storeValue: "gsk_external_reload" });

    expect(result.current.localKey).toBe("gsk_external_reload");
  });
});
