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
  getDataDirectory: vi.fn().mockResolvedValue({
    active: "/default/app/data",
    default: "/default/app/data",
    pending: null,
    lastError: null,
  }),
  pickDataDirectory: vi.fn().mockResolvedValue(null),
  setDataDirectory: vi.fn().mockResolvedValue(undefined),
  restartApp: vi.fn().mockResolvedValue(undefined),
  setFloatingWidgetEnabled: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../assets/icons/hourglass.svg?react", () => ({
  default: () => null,
}));
vi.mock("../assets/icons/sparkle.svg?react", () => ({
  default: () => null,
}));
vi.mock("../assets/icons/scroll-text.svg?react", () => ({
  default: () => null,
}));
