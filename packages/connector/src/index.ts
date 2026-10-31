/* eslint-disable no-await-in-loop, no-unmodified-loop-condition -- A sequential worker must finish one leased mutation before claiming the next; signals stop it. */
import { readFile } from "node:fs/promises";

import {
  Client,
  StreamableHTTPClientTransport,
} from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

type Config = {
  name: string;
  hive: string;
  keyFile: string;
  connectionId?: string;
} & (
  | {
      transport: "stdio";
      command: string;
      args: string[];
      env?: Record<string, string>;
    }
  | { transport: "http"; url: string; headers?: Record<string, string> }
);
const config = JSON.parse(
  await readFile(process.argv[2] ?? "hive-connector.json", "utf-8")
) as Config;
const hive = new URL(config.hive);
if (
  hive.protocol !== "https:" &&
  !(
    hive.protocol === "http:" &&
    ["localhost", "127.0.0.1"].includes(hive.hostname)
  )
) {
  throw new Error("Hive requires HTTPS, except loopback development");
}
if (hive.username || hive.password) {
  throw new Error("Use keyFile for Hive authentication");
}
const keyContents = await readFile(config.keyFile, "utf-8");
const key = keyContents.trim();
const rpc = async (name: string, args: Record<string, unknown>) => {
  const response = await fetch(hive, {
    body: JSON.stringify({
      id: crypto.randomUUID(),
      jsonrpc: "2.0",
      method: "tools/call",
      params: { arguments: args, name },
    }),
    headers: {
      Accept: "application/json, text/event-stream",
      Authorization: `Bearer ${key}`,
      "Content-Type": "application/json",
      "MCP-Protocol-Version": "2025-11-25",
    },
    method: "POST",
    redirect: "error",
    signal: AbortSignal.timeout(40_000),
  });
  if (!response.ok) {
    throw new Error(`Hive request failed (${response.status})`);
  }
  const value = (await response.json()) as {
    error?: unknown;
    result?: { isError?: boolean; structuredContent?: Record<string, unknown> };
  };
  if (
    value.error ||
    value.result?.isError ||
    !value.result?.structuredContent
  ) {
    throw new Error("Hive connector operation failed");
  }
  return value.result.structuredContent;
};
// Only local, explicitly trusted configuration can select executables or upstream URLs.
// Hive credentials are never passed to the upstream process or remote MCP.
const transport =
  config.transport === "stdio"
    ? new StdioClientTransport({
        args: config.args,
        command: config.command,
        env: config.env ?? {},
        maxBufferSize: 524_288,
        stderr: "ignore",
      })
    : new StreamableHTTPClientTransport(new URL(config.url), {
        requestInit: { headers: config.headers ?? {}, redirect: "error" },
      });
const client = new Client({ name: "hive-connector", version: "1.0.0" });
await client.connect(transport);
const { tools } = await client.listTools();
const allowed = new Set(tools.map((tool) => tool.name));
let { connectionId } = config;
if (!connectionId) {
  const registered = await rpc("register_mcp_connection", {
    name: config.name,
    tools,
  });
  if (typeof registered.id !== "string") {
    throw new TypeError("Missing connection ID");
  }
  connectionId = registered.id;
}
console.log(
  `Connected ${config.name}: ${connectionId}. Save connectionId in your config to reuse it.`
);
let running = true;
process.on("SIGINT", () => {
  running = false;
});
process.on("SIGTERM", () => {
  running = false;
});
try {
  while (running) {
    const { job } = await rpc("poll_mcp", { connection_id: connectionId });
    if (!job) {
      await Bun.sleep(1000);
      continue;
    }
    const claimed = job as {
      id: string;
      lease: string;
      request: { name: string; arguments: Record<string, unknown> };
    };
    let result;
    try {
      if (!allowed.has(claimed.request.name)) {
        throw new Error("Tool not in local allowlist");
      }
      result = await client.callTool(claimed.request, { timeout: 25_000 });
      if (JSON.stringify(result).length > 524_288) {
        throw new Error("Result exceeds Hive limit");
      }
    } catch {
      result = {
        content: [
          {
            text: "Upstream MCP call failed or exceeded limits. Execution outcome may be unknown; do not retry mutations automatically.",
            type: "text",
          },
        ],
        isError: true,
      };
    }
    // No retries: repeating a mutation after an uncertain transport failure is unsafe.
    await rpc("complete_mcp", {
      connection_id: connectionId,
      job_id: claimed.id,
      lease: claimed.lease,
      result,
    });
  }
} finally {
  await client.close();
}
