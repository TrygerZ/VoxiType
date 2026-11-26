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

import {
  ackWidgetHide,
  ackWidgetReveal,
  deletePerAppMode,
  getActiveApp,
  getPerAppModes,
  listRunningApps,
  setPerAppMode,
  testOllama,
} from "./tauri";

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

describe("testOllama wrapper", () => {
  it("invokes test_ollama with the requested model", async () => {
    await testOllama("qwen2.5:3b");
    expect(invokeMock).toHaveBeenCalledWith("test_ollama", {
      model: "qwen2.5:3b",
    });
  });
});

describe("per-app wrappers", () => {
  it("invokes typed per-app commands with matching arguments", async () => {
    const mapping = {
      id: 0,
      app_process_name: "code",
      app_display_name: "VS Code",
      mode_id: "dictation",
    };

    await getPerAppModes();
    expect(invokeMock).toHaveBeenCalledWith("get_per_app_modes");

    await setPerAppMode(mapping);
    expect(invokeMock).toHaveBeenCalledWith("set_per_app_mode", { mapping });

    await deletePerAppMode(42);
    expect(invokeMock).toHaveBeenCalledWith("delete_per_app_mode", { id: 42 });

    await getActiveApp();
    expect(invokeMock).toHaveBeenCalledWith("get_active_app");

    await listRunningApps();
    expect(invokeMock).toHaveBeenCalledWith("list_running_apps");
  });
});
