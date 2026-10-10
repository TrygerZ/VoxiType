import { beforeEach, describe, expect, it } from "vitest";

import { useToastStore } from "./toastStore";

describe("toastStore.add", () => {
  beforeEach(() => {
    useToastStore.setState({ toasts: [] });
  });

  it("collapses an identical message and type into one toast", () => {
    const { add } = useToastStore.getState();

    add("save failed", "error");
    add("save failed", "error");

    expect(useToastStore.getState().toasts).toHaveLength(1);
  });

  it("adds a toast when the type differs", () => {
    const { add } = useToastStore.getState();

    add("save failed", "error");
    add("save failed", "info");

    expect(useToastStore.getState().toasts).toHaveLength(2);
  });

  it("adds a toast when the message differs", () => {
    const { add } = useToastStore.getState();

    add("save failed", "error");
    add("load failed", "error");

    expect(useToastStore.getState().toasts).toHaveLength(2);
  });

  it("allows the same message again after the previous toast is removed", () => {
    const { add } = useToastStore.getState();

    add("save failed", "error");
    const firstId = useToastStore.getState().toasts[0].id;
    useToastStore.getState().remove(firstId);
    add("save failed", "error");

    expect(useToastStore.getState().toasts).toHaveLength(1);
  });
});
