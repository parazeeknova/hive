import { expect, test } from "bun:test";

import { withinResultLimit } from "../../packages/connector/src/limits";

test("connector bounds JSON wire bytes rather than UTF-16 characters", () => {
  expect(withinResultLimit({ text: "a".repeat(500_000) })).toBe(true);
  const unicode = { text: "🙂".repeat(140_000) };
  expect(JSON.stringify(unicode).length).toBeLessThan(524_288);
  expect(withinResultLimit(unicode)).toBe(false);
  expect(withinResultLimit({ text: "é".repeat(262_144) })).toBe(false);
});
