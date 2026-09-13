import { describe, expect, it } from "vitest";
import {
  getBooleanSetting,
  getHotkeySetting,
  getStringSetting,
  isBoolean,
  isHotkeyConfig,
  isString,
} from "./settingsGuards";

describe("settingsGuards", () => {
  it("validates string and boolean values", () => {
    expect(isString("hello")).toBe(true);
    expect(isString(123)).toBe(false);
    expect(isBoolean(true)).toBe(true);
    expect(isBoolean("true")).toBe(false);
  });

  it("validates hotkey config shapes", () => {
    expect(isHotkeyConfig({ key: "Ctrl+Space", mode: "ptt" })).toBe(true);
    expect(isHotkeyConfig({ key: "Ctrl+Space" })).toBe(false);
    expect(isHotkeyConfig(null)).toBe(false);
    expect(isHotkeyConfig(undefined)).toBe(false);
    expect(isHotkeyConfig("Ctrl+Space")).toBe(false);
  });

  it("provides fallback for string settings", () => {
    expect(getStringSetting("en", "id")).toBe("en");
    expect(getStringSetting(null, "id")).toBe("id");
    expect(getStringSetting(undefined, "id")).toBe("id");
  });

  it("provides fallback for boolean settings", () => {
    expect(getBooleanSetting(true, false)).toBe(true);
    expect(getBooleanSetting(false, true)).toBe(false);
    expect(getBooleanSetting(null, true)).toBe(true);
    expect(getBooleanSetting(undefined, false)).toBe(false);
  });

  it("provides fallback for hotkey settings", () => {
    const fallback = { key: "Ctrl+Space", mode: "ptt" };
    expect(
      getHotkeySetting({ key: "Alt+Space", mode: "toggle" }, fallback),
    ).toEqual({
      key: "Alt+Space",
      mode: "toggle",
    });
    expect(getHotkeySetting(null, fallback)).toEqual(fallback);
    expect(getHotkeySetting(undefined, fallback)).toEqual(fallback);
    expect(getHotkeySetting({ key: "Ctrl+Space" }, fallback)).toEqual(fallback);
  });
});
