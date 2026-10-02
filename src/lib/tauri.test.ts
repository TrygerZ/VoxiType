import { describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn().mockResolvedValue(undefined),
  listenMock: vi.fn().mockResolvedValue(() => undefined),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));
// The global test setup mocks this module wholesale; unmock it so the real
// invoke wrappers under test are exercised.
vi.unmock("./tauri");

import { ackWidgetHide, ackWidgetReveal } from "./tauri";

describe("widget acknowledgement wrappers", () => {
  it("invokes ack_widget_reveal with the payload id", async () => {
    await ackWidgetReveal(7);
    expect(invokeMock).toHaveBeenCalledWith("ack_widget_reveal", { id: 7 });
  });

  it("invokes ack_widget_hide with the payload id", async () => {
    await ackWidgetHide(3);
    expect(invokeMock).toHaveBeenCalledWith("ack_widget_hide", { id: 3 });
  });
});
