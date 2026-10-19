import { describe, expect, it } from "vitest";
import source from "./floating.tsx?raw";

/**
 * The overlay window is created hidden and revealed by the backend's
 * page-load hook. Revealing from the page must not wait on
 * requestAnimationFrame: WebKit suspends it while the window is hidden, so
 * the widget never appears on macOS or Linux.
 */
describe("floating entry point", () => {
  it("never gates the reveal on requestAnimationFrame", () => {
    expect(source).not.toMatch(/requestAnimationFrame\s*\(/);
  });
});
