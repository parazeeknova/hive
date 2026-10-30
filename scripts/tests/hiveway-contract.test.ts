/// <reference lib="dom" />
import { afterEach, describe, expect, mock, spyOn, test } from "bun:test";

import { capabilityDisplayName } from "../../apps/nectar/lib/capability-name";
import {
  atomicMoney,
  hiveway,
  parsePrice,
  validatePublishText,
  validateRepository,
} from "../../apps/nectar/lib/hiveway";

afterEach(() => {
  mock.restore();
});

describe("dashboard contract boundaries", () => {
  test("keeps six-decimal atomic prices exact", () => {
    expect(parsePrice("0")).toBe("0");
    expect(parsePrice("0.000001")).toBe("1");
    expect(parsePrice("0.29")).toBe("290000");
    expect(parsePrice("1000")).toBe("1000000000");
    expect(atomicMoney("10001")).toBe("0.010001");
    expect(atomicMoney("1000000000")).toBe("1000.00");
    for (const value of ["0.0000001", "1e2", "-1", "1000.000001", "NaN"]) {
      expect(() => parsePrice(value)).toThrow();
    }
  });

  test("requires a public GitHub URL and immutable commit", () => {
    const revision = "a".repeat(40);
    expect(() =>
      validateRepository("https://github.com/owner/repo", revision)
    ).not.toThrow();
    for (const url of [
      "http://github.com/owner/repo",
      "https://user@github.com/owner/repo",
      "https://github.com/owner/repo?token=x",
      "https://github.com/owner/repo/tree/main",
      "https://gitlab.com/owner/repo",
    ]) {
      expect(() => validateRepository(url, revision)).toThrow();
    }
    expect(() =>
      validateRepository("https://github.com/owner/repo", "main")
    ).toThrow();
  });

  test("validates UTF-8 byte limits rather than character counts", () => {
    expect(() => validatePublishText("🌻".repeat(21), "Report")).toThrow();
    expect(() => validatePublishText("Report", "🌻".repeat(251))).toThrow();
    expect(() => validatePublishText("   ", "Report")).toThrow();
  });

  test("reads progress and terminal receipt bodies regardless of HTTP success", async () => {
    const fetch = spyOn(globalThis, "fetch");
    const receipt = {
      capability_id: "capability",
      id: "invocation",
      state: "execution_failed",
    };
    fetch.mockResolvedValueOnce(Response.json(receipt, { status: 409 }));
    expect(await hiveway("/v1/invocations/invocation", "credential")).toEqual(
      receipt
    );
    fetch.mockResolvedValueOnce(
      Response.json({ state: "settling" }, { status: 202 })
    );
    expect(await hiveway("/v1/invocations/invocation", "credential")).toEqual({
      state: "settling",
    });
    fetch.mockResolvedValueOnce(
      Response.json({ x402Version: 2 }, { status: 402 })
    );
    expect(await hiveway("/v1/invocations/invocation", "credential")).toEqual({
      x402Version: 2,
    });
    const init = fetch.mock.calls[0]?.[1];
    expect(new Headers(init?.headers).get("Authorization")).toBe(
      "Bearer credential"
    );
    expect(init?.cache).toBe("no-store");
  });

  test("does not confuse idempotency conflicts with terminal receipts", async () => {
    const fetch = spyOn(globalThis, "fetch");
    fetch.mockResolvedValueOnce(
      Response.json(
        { error: "idempotency_key_reused_with_different_input" },
        { status: 409 }
      )
    );
    await expect(hiveway("/v1/quotes")).rejects.toThrow(
      "idempotency_key_reused_with_different_input"
    );
    fetch.mockResolvedValueOnce(
      new Response("Origin rejected", { status: 403 })
    );
    await expect(hiveway("/v1/quotes")).rejects.toThrow("Origin rejected");
    fetch.mockResolvedValueOnce(new Response(null, { status: 503 }));
    await expect(hiveway("/v1/quotes")).rejects.toThrow("503");
  });
});

test("legacy demo titles are readable while custom creator names stay unchanged", () => {
  const id = "54984039-960d-4bf9-be06-f4e69a3b09d4";
  expect(capabilityDisplayName(`Local repository stack ${id}`)).toBe(
    "Repository stack report"
  );
  expect(capabilityDisplayName(`Hosted echo ${id}`)).toBe("JSON echo");
  expect(capabilityDisplayName(`Free MCP echo ${id}`)).toBe("Agent echo");
  expect(capabilityDisplayName(`Custom tool ${id}`)).toBe(`Custom tool ${id}`);
  expect(capabilityDisplayName("Repository stack report")).toBe(
    "Repository stack report"
  );
});
