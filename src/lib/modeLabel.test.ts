import { describe, expect, it } from "vitest";

import { en, id, modeLabel } from "./i18n";

const tEn = (key: string) => en[key] ?? key;
const tId = (key: string) => id[key] ?? key;

describe("modeLabel", () => {
  it("translates the built-in mode ids", () => {
    expect(modeLabel(tEn, "dictation")).toBe("Dictation");
    expect(modeLabel(tEn, "message")).toBe("Message");
    expect(modeLabel(tEn, "email")).toBe("Email");
    expect(modeLabel(tEn, "file")).toBe("File");
    expect(modeLabel(tId, "dictation")).toBe("Dikte");
  });

  it("capitalizes an unknown mode id instead of showing a raw i18n key", () => {
    expect(modeLabel(tEn, "custom")).toBe("Custom");
    expect(modeLabel(tEn, "summary")).toBe("Summary");
  });
});
