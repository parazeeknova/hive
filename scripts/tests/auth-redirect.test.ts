import { describe, expect, test } from "bun:test";

import {
  dashboardDestination,
  dashboardReturnPath,
} from "../../apps/nectar/lib/auth-redirect";
import { readOAuthHandshake } from "../../apps/nectar/lib/oauth";

describe("landing search through authentication", () => {
  test("preserves search terms and the Discover view", () => {
    const path = dashboardDestination({
      q: "repo & JSON café",
      view: "discover",
    });
    const url = new URL(path, "https://hive.invalid");
    expect(url.searchParams.get("q")).toBe("repo & JSON café");
    expect(url.searchParams.get("view")).toBe("discover");
    const handshake = JSON.stringify({
      next: path,
      state: "state",
      verifier: "a".repeat(43),
    });
    expect(readOAuthHandshake(handshake, "state", "code").next).toBe(path);
  });
  test("rejects external and non-dashboard OAuth return destinations", () => {
    for (const value of [
      "https://evil.invalid/dashboard",
      "//evil.invalid/dashboard",
      "/api/auth/logout",
      "/dashboard/../api/auth/logout",
      // oxlint-disable-next-line no-script-url -- Reject script URLs at the OAuth redirect boundary.
      "javascript:alert(1)",
      "/dashboard/extra",
      null,
      `/dashboard?${"a".repeat(2048)}`,
    ]) {
      expect(dashboardReturnPath(value)).toBe("/dashboard");
    }
  });
  test("keeps old OAuth handshakes compatible and rejects forged targets", () => {
    for (const next of [undefined, "https://evil.invalid/dashboard"]) {
      expect(
        readOAuthHandshake(
          JSON.stringify({ next, state: "state", verifier: "a".repeat(43) }),
          "state",
          "code"
        ).next
      ).toBe("/dashboard");
    }
  });
});
