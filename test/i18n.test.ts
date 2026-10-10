import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { en, id } from "../src/lib/i18n";

// Catalogs are flat today; flatten() keeps the parity check honest if a
// nested group is ever introduced (AGENTS.md machine rule).
function flatten(source: Record<string, unknown>, prefix = ""): Map<string, string> {
  const out = new Map<string, string>();
  for (const [key, value] of Object.entries(source)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "string") {
      out.set(path, value);
    } else if (value && typeof value === "object") {
      for (const [k, v] of flatten(value as Record<string, unknown>, path)) {
        out.set(k, v);
      }
    }
  }
  return out;
}

// U+2013 en dash and U+2014 em dash, built from code points so this source
// carries neither character itself.
const EN_DASH = String.fromCharCode(0x2013);
const EM_DASH = String.fromCharCode(0x2014);
const FORBIDDEN_DASH = new RegExp(`[${EN_DASH}${EM_DASH}]`);

describe("i18n catalogs", () => {
  const catalogs = { id, en };

  it("id and en have identical key sets", () => {
    const idKeys = [...flatten(id).keys()].sort();
    const enKeys = [...flatten(en).keys()].sort();
    const missingInEn = idKeys.filter((k) => !enKeys.includes(k));
    const missingInId = enKeys.filter((k) => !idKeys.includes(k));
    expect({ missingInEn, missingInId }).toEqual({
      missingInEn: [],
      missingInId: [],
    });
  });

  it.each(Object.keys(catalogs))("%s has no en/em dash in any value", (lang) => {
    const offenders = [...flatten(catalogs[lang as keyof typeof catalogs])]
      .filter(([, value]) => FORBIDDEN_DASH.test(value))
      .map(([key]) => key);
    expect(offenders).toEqual([]);
  });

  it("i18n.ts source has no en/em dash anywhere, comments included", () => {
    const source = readFileSync(resolve(process.cwd(), "src/lib/i18n.ts"), "utf8");
    expect(FORBIDDEN_DASH.test(source)).toBe(false);
  });
});
