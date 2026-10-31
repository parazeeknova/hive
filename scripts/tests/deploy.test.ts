import { describe, expect, test } from "bun:test";

import { Dokploy, immutableImage, requestJson, verifyHealth } from "../deploy";
import type { Runtime } from "../deploy";

const fixture = (responses: unknown[]) => {
  const calls: string[] = [];
  const sleeps: number[] = [];
  let elapsed = 0;
  const io: Runtime = {
    fetch: ((url: string | URL | Request) => {
      calls.push(String(url));
      const response = responses.shift();
      return Promise.resolve(
        new Response(response === "" ? "" : JSON.stringify(response), {
          status: 200,
        })
      );
    }) as typeof fetch,
    now: () => elapsed,
    sleep: (milliseconds) => {
      elapsed += milliseconds;
      sleeps.push(milliseconds);
      return Promise.resolve();
    },
  };
  return { calls, io, sleeps };
};
describe("immutable releases and Dokploy queues", () => {
  test("health probes identify Hive to Cloudflare and never send the Dokploy key", async () => {
    const io: Runtime = {
      fetch: ((_url: string | URL | Request, options?: RequestInit) => {
        expect(new Headers(options?.headers).get("User-Agent")).toBe(
          "Hive-deploy/1.0"
        );
        expect(new Headers(options?.headers).has("x-api-key")).toBe(false);
        return Promise.resolve(
          Response.json({ revision: "new", status: "ok" })
        );
      }) as typeof fetch,
      now: () => 0,
      sleep: () => Promise.resolve(),
    };
    await verifyHealth("https://api.example", "new", false, 100, io);
  });
  test("accepts empty mutations and JSON reads", async () => {
    const { io } = fixture(["", { status: "ok" }]);
    expect(
      await new Dokploy("https://dokploy.example", "private", io).api(
        "application.deploy",
        {}
      )
    ).toBeNull();
    expect(
      await requestJson("https://dokploy.example/api/application.one", {}, io)
    ).toEqual({ status: "ok" });
  });
  test("retries empty health and readiness responses", async () => {
    const health = { revision: "new", status: "ok" };
    const { io, sleeps, calls } = fixture([
      "",
      health,
      "",
      health,
      { status: "ok" },
    ]);
    await verifyHealth("https://api.example", "new", false, 240_000, io);
    expect(sleeps).toEqual([10_000, 10_000]);
    expect(calls.at(-1)).toBe("https://api.example/api/ready");
  });
  test("health timeouts retain the final readiness, mismatch and stale-revision failure", async () => {
    const cases = [
      {
        expected: "Backend dependencies are not ready",
        frontend: false,
        responses: [
          { revision: "new", status: "ok" },
          { status: "unavailable" },
        ],
      },
      {
        expected: "Frontend is connected to a different backend release",
        frontend: true,
        responses: [
          { hiveway: { revision: "old" }, nectar: "ok", revision: "new" },
        ],
      },
      {
        expected: "expected new, received old",
        frontend: false,
        responses: [{ revision: "old", status: "ok" }],
      },
    ];
    await Promise.all(
      cases.map(async (item) => {
        const { io, sleeps } = fixture(item.responses);
        await expect(
          verifyHealth(
            "https://service.example",
            "new",
            item.frontend,
            10_000,
            io
          )
        ).rejects.toThrow(item.expected);
        expect(sleeps).toEqual([10_000]);
      })
    );
    const { io } = fixture([]);
    await expect(
      verifyHealth("https://service.example", "new", false, 0, io)
    ).rejects.toThrow("no valid response");
  });
  test("requires matching immutable image digests", () => {
    const image = `ghcr.io/parazeeknova/hive-nectar@sha256:${"a".repeat(64)}`;
    expect(immutableImage(image, "nectar")).toBe(image);
    for (const invalid of [
      image.replace("nectar", "hiveway"),
      "ghcr.io/parazeeknova/hive-nectar:main",
      `${image}x`,
    ]) {
      expect(() => immutableImage(invalid, "nectar")).toThrow();
    }
  });
  test("old success cannot finish a new deployment", async () => {
    const old = { deploymentId: "old", status: "done", title: "release" };
    const current = { deploymentId: "new", status: "done", title: "release" };
    const { io, sleeps } = fixture([[old], "", "", [old], [current, old]]);
    await new Dokploy("https://dokploy.example", "private", io).deploy(
      "app",
      "image",
      "release"
    );
    expect(sleeps).toEqual([10_000]);
  });
  test("reports deployment failures and requires matching frontend/backend revisions", async () => {
    const failed = fixture([
      [],
      "",
      "",
      [{ deploymentId: "new", status: "error", title: "release" }],
    ]);
    await expect(
      new Dokploy("https://dokploy.example", "private", failed.io).deploy(
        "app",
        "image",
        "release"
      )
    ).rejects.toThrow("failed");
    const wrong = {
      hiveway: { revision: "old" },
      nectar: "ok",
      revision: "new",
    };
    const right = { ...wrong, hiveway: { revision: "new" } };
    const { io, sleeps } = fixture([wrong, right]);
    await verifyHealth("https://hive.example", "new", true, 240_000, io);
    expect(sleeps).toEqual([10_000]);
  });
});
