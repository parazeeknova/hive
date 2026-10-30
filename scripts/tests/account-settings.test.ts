import { describe, expect, test } from "bun:test";

import { readOAuthHandshake } from "../../apps/nectar/lib/oauth";
import { sampleInput } from "../../apps/nectar/lib/sample-input";

describe("OAuth browser binding", () => {
  const raw = JSON.stringify({
    state: "random-state",
    verifier: "a".repeat(43),
  });
  test("accepts a matching state and PKCE verifier", () => {
    expect(readOAuthHandshake(raw, "random-state", "code").verifier).toBe(
      "a".repeat(43)
    );
  });
  test("rejects missing, forged, corrupt and mismatched callbacks", () => {
    for (const [cookie, state, code] of [
      [undefined, "random-state", "code"],
      [raw, "attacker-state", "code"],
      [raw, "random-state", null],
      ["{", "random-state", "code"],
      [
        JSON.stringify({ state: "random-state", verifier: "short" }),
        "random-state",
        "code",
      ],
    ] as const) {
      expect(() => readOAuthHandshake(cookie, state, code)).toThrow();
    }
  });
});

test("sample input follows published schema examples and nested property types", () => {
  expect(
    sampleInput({
      properties: {
        count: { minimum: 3, type: "integer" },
        items: { items: { const: true }, minItems: 2, type: "array" },
        kind: { enum: ["echo", "other"] },
        name: { type: "string" },
      },
      type: "object",
    })
  ).toEqual({ count: 3, items: [true, true], kind: "echo", name: "sample" });
  expect(sampleInput({ examples: [{ value: "hello" }] })).toEqual({
    value: "hello",
  });
});

test("sample strings satisfy bounded minimum lengths", () => {
  expect(sampleInput({ minLength: 12, type: "string" })).toHaveLength(12);
  expect(sampleInput({ minLength: 100_000, type: "string" })).toHaveLength(
    4096
  );
});
