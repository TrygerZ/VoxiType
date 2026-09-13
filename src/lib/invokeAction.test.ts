import { beforeEach, describe, expect, it, vi } from "vitest";

import { useToastStore } from "../stores/toastStore";
import { invokeAction } from "./invokeAction";

describe("invokeAction", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useToastStore.setState({ toasts: [] });
  });

  it("returns true and does not toast on success", async () => {
    const fn = vi.fn().mockResolvedValue("success");

    const result = await invokeAction(fn);

    expect(result).toBe(true);
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it("shows error toast and calls onError on rejection", async () => {
    const fn = vi.fn().mockRejectedValue(new Error("IPC network failure"));
    const onError = vi.fn();

    const result = await invokeAction(fn, onError);

    expect(result).toBe(false);
    expect(onError).toHaveBeenCalledWith(expect.any(Error));

    const toasts = useToastStore.getState().toasts;
    expect(toasts).toHaveLength(1);
    expect(toasts[0].message).toBe("IPC network failure");
    expect(toasts[0].type).toBe("error");
  });
});
