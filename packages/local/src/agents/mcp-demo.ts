//! Live MCP acceptance against a disposable Hive instance; only public Anvil wallets.
import { chmod, writeFile } from "node:fs/promises";

import { localAccount } from "../config/chain";
import { HiveAgent } from "./client";

const base = process.env.HIVE_DEMO_URL ?? "http://127.0.0.1:3001";
const creator = await HiveAgent.connect(base, localAccount(1), [
  "capabilities:read",
  "capabilities:publish",
  "capabilities:invoke",
  "receipts:read",
]);
const buyer = await HiveAgent.connect(base, localAccount(2), [
  "capabilities:read",
  "capabilities:invoke",
  "receipts:read",
]);
let sequence = 0;
const rpc = async (agent: HiveAgent, method: string, params: unknown = {}) => {
  sequence += 1;
  const response = await agent.json<{
    error?: unknown;
    result: {
      isError?: boolean;
      structuredContent?: Record<string, unknown>;
      tools?: { name: string }[];
      protocolVersion?: string;
    };
  }>("/mcp", { id: sequence, jsonrpc: "2.0", method, params });
  if (response.error || response.result.isError) {
    throw new Error(JSON.stringify(response));
  }
  return response.result;
};
const call = async (agent: HiveAgent, name: string, args: unknown) => {
  const result = await rpc(agent, "tools/call", { arguments: args, name });
  if (!result.structuredContent) {
    throw new Error("Missing tool result");
  }
  return result.structuredContent;
};
const init = await rpc(buyer, "initialize", {
  capabilities: {},
  clientInfo: { name: "hive-acceptance", version: "1" },
  protocolVersion: "2025-11-25",
});
if (init.protocolVersion !== "2025-11-25") {
  throw new Error("MCP negotiation failed");
}
const schema = {
  additionalProperties: false,
  properties: { message: { type: "string" } },
  required: ["message"],
  type: "object",
};
const artifact = await call(creator, "upload_artifact", {
  input_schema: schema,
  output_schema: schema,
  wasm_base64: Buffer.from(
    await Bun.file(
      new URL("../../../../.hive/local/echo.wasm", import.meta.url)
    ).arrayBuffer()
  ).toString("base64"),
});
const capability = await call(creator, "publish_capability", {
  artifact_id: artifact.id,
  description: "Live free MCP acceptance",
  name: `Free MCP echo ${crypto.randomUUID()}`,
  price: "0",
  runtime: "wasm",
});
const tool = `hive_${String(capability.id).replaceAll("-", "")}`;
const listed = await rpc(buyer, "tools/list");
if (!listed.tools?.some((t) => t.name === tool)) {
  throw new Error("Published tool absent");
}
const args = {
  idempotency_key: crypto.randomUUID(),
  input: { message: "agent invokes hosted artifact mid-task" },
};
const invoked = await call(buyer, tool, args);
const replay = await call(buyer, tool, args);
if (
  invoked.invocation_id !== replay.invocation_id ||
  JSON.stringify((invoked.data as { result: unknown }).result) !==
    JSON.stringify(args.input)
) {
  throw new Error("Free result or replay incorrect");
}
const foreign = await rpc(creator, "tools/call", {
  arguments: { invocation_id: invoked.invocation_id },
  name: "get_receipt",
}).then(
  () => false,
  () => true
);
if (!foreign) {
  throw new Error("Cross-credential receipt access allowed");
}
const changed = await rpc(buyer, "tools/call", {
  arguments: { ...args, input: { message: "tampered replay" } },
  name: tool,
}).then(
  () => false,
  () => true
);
if (!changed) {
  throw new Error("Changed-input replay allowed");
}
const times: number[] = [];
for (let i = 0; i < 30; i += 1) {
  const start = performance.now();
  // oxlint-disable-next-line no-await-in-loop -- Measure sequential request latency.
  await call(buyer, "search_capabilities", { q: "Free MCP echo" });
  times.push(performance.now() - start);
}
times.sort((a, b) => a - b);
const file = new URL("../../../../.hive/local/opencode-token", import.meta.url);
await writeFile(file, buyer.token, { mode: 0o600 });
await chmod(file, 0o600);
console.log(
  JSON.stringify({
    artifact: artifact.id,
    capability: capability.id,
    freeInvocation: invoked.invocation_id,
    mcpSearchMs: {
      max: times[29],
      median: times[15],
      min: times[0],
      p95: times[28],
    },
    replaySafe: true,
  })
);
