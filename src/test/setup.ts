import "@testing-library/jest-dom/vitest";

import { vi } from "vitest";

vi.mock("../lib/tauri", () => ({
  formatTauriError: (error: unknown) => String(error),
  getSettings: vi.fn().mockResolvedValue({}),
  updateSetting: vi.fn().mockResolvedValue(undefined),
  setWhisperCppPaths: vi.fn().mockResolvedValue(undefined),
  setHotkey: vi.fn().mockResolvedValue(undefined),
  onEvent: vi.fn().mockResolvedValue(() => undefined),
  invoke: vi.fn().mockResolvedValue(undefined),
}));
