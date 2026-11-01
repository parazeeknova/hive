import { expect, test } from "bun:test";

import {
  capabilityInput,
  usesJsonInput,
} from "../../apps/nectar/lib/capability-input";
import type { RegistryCapability } from "../../apps/nectar/lib/hiveway";

const base: RegistryCapability = {
  active: true,
  creator_wallet: "github:fixture",
  definition: {},
  description: "Fixture",
  id: "fixture",
  name: "Fixture",
  price: "0",
  runtime: "repository_stack",
  version: "1",
};

test("connected tools and MCP workflows use editable JSON bindings without repository fields", () => {
  for (const tool of [
    { ...base, runtime: "connected_mcp" as const },
    {
      ...base,
      definition: { mcp_tools: ["read_file"] },
      runtime: "workflow" as const,
    },
  ]) {
    expect(usesJsonInput(tool)).toBe(true);
    expect(JSON.stringify(capabilityInput(tool))).toContain("connection_id");
    expect(JSON.stringify(capabilityInput(tool))).not.toContain(
      "repository_url"
    );
    expect(JSON.stringify(capabilityInput(tool))).not.toContain("revision");
  }
  expect(usesJsonInput(base)).toBe(false);
  expect(usesJsonInput({ ...base, runtime: "workflow" })).toBe(false);
  expect(usesJsonInput({ ...base, runtime: "wasm" })).toBe(true);
  expect(usesJsonInput({ ...base, runtime: "repository_scaffold" })).toBe(true);
});
