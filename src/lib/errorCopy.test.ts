import { describe, expect, it } from "vitest";

import { errorCopy } from "./errorCopy";

describe("errorCopy", () => {
  it("maps a rejected API key to localized copy and the STT settings tab", () => {
    const copy = errorCopy("SttApiKeyInvalid", "HTTP 401: invalid_api_key");
    expect(copy.settingsTab).toBe("stt");
    expect(copy.message).not.toContain("401");
  });

  it("falls back to the backend message for unknown codes", () => {
    expect(errorCopy("SomethingNew", "raw detail")).toEqual({
      message: "raw detail",
      settingsTab: undefined,
    });
  });
});
