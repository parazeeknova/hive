import { expect, test } from "bun:test";

import { renderToStaticMarkup } from "react-dom/server";

import Docs from "../../apps/nectar/app/(marketing)/docs/page";
import { clients } from "../../apps/nectar/app/(marketing)/mcp-section";

test("copied setup commands preserve credential references through the shell", async () => {
  const sample = "fixture-secret-that-must-not-enter-process-arguments";
  const results = await Promise.all(
    clients
      .filter((client) => client.id !== "codex")
      .map(async (client) => {
        const child = Bun.spawn(
          [
            "sh",
            "-c",
            `claude() { printf '%s\\n' "$@"; }; opencode() { printf '%s\\n' "$@"; }; ${client.command}`,
          ],
          {
            env: { ...process.env, HIVE_API_KEY: sample },
            stderr: "pipe",
            stdout: "pipe",
          }
        );
        const arguments_ = await new Response(child.stdout).text();
        expect(await child.exited).toBe(0);
        expect(arguments_).not.toContain(sample);
        return { arguments_, id: client.id };
      })
  );
  expect(
    results.find((client) => client.id === "claude")?.arguments_
    // oxlint-disable-next-line no-template-curly-in-string -- Assert the literal reference survives shell parsing.
  ).toContain("Bearer ${HIVE_API_KEY}");
  expect(
    results.find((client) => client.id === "opencode")?.arguments_
  ).toContain("Bearer {env:HIVE_API_KEY}");
});

test("inline scaffold and build instructions preserve word boundaries", () => {
  const text = renderToStaticMarkup(<Docs />)
    .replaceAll(/<[^>]*>/gu, "")
    .replaceAll(/\s+/gu, " ");
  expect(text).toContain("my-app with HIVE_API_KEY");
  expect(text).toContain("call build_capability");
  expect(text).toContain("UUID idempotency_key");
});
