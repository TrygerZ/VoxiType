export interface HotkeyConfig {
  key: string;
  mode: string;
}

export function isString(value: unknown): value is string {
  return typeof value === "string";
}

export function isBoolean(value: unknown): value is boolean {
  return typeof value === "boolean";
}

export function isHotkeyConfig(value: unknown): value is HotkeyConfig {
  return (
    typeof value === "object" &&
    value !== null &&
    "key" in value &&
    typeof value.key === "string" &&
    "mode" in value &&
    typeof value.mode === "string"
  );
}

export function getStringSetting(value: unknown, fallback: string): string {
  return typeof value === "string" ? value : fallback;
}

export function getBooleanSetting(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

export function getHotkeySetting(
  value: unknown,
  fallback: HotkeyConfig,
): HotkeyConfig {
  return isHotkeyConfig(value) ? value : fallback;
}
