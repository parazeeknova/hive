import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
/* eslint-disable no-await-in-loop, no-unmodified-loop-condition -- A sequential worker must finish one leased mutation before claiming the next; signals stop it. */
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  Client,
  StreamableHTTPClientTransport,
} from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

import { Directories } from "./directories";
import { withinResultLimit } from "./limits";

type Config = {
  name: string;
  hive: string;
  keyFile: string;
  connectionId?: string;
  workingDirectory?: string;
} & (
  | {
      transport: "stdio";
      command: string;
      args: string[];
      env?: Record<string, string>;
    }
  | { transport: "filesystem" }
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
class FatalHiveError extends Error {
  override name = "FatalHiveError";
}
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
  if (response.status === 401 || response.status === 403) {
    throw new FatalHiveError("Hive credentials rejected");
  }
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
    if (
      ["forbidden", "invalid_connection_id"].includes(
        String(value.result?.structuredContent?.error)
      )
    ) {
      throw new FatalHiveError("Hive connection is unavailable or revoked");
    }
    throw new Error("Hive connector operation failed");
  }
  return value.result.structuredContent;
};
// Only local, explicitly trusted configuration can select executables or upstream URLs.
// Hive credentials are never passed to the upstream process or remote MCP.
const directories = new Directories(config.keyFile);
if (config.transport === "filesystem") {
  await directories.allow(config.workingDirectory ?? process.cwd());
}
const quarantine =
  config.transport === "filesystem"
    ? await mkdtemp(path.join(os.tmpdir(), "hive-filesystem-"))
    : null;
let transport;
if (config.transport === "filesystem") {
  transport = new StdioClientTransport({
    args: [
      fileURLToPath(
        import.meta
          .resolve("@modelcontextprotocol/server-filesystem/dist/index.js")
      ),
      ...directories.approved,
      ...(quarantine ? [quarantine] : []),
    ],
    command: process.execPath,
    env: {},
    maxBufferSize: 524_288,
    stderr: "ignore",
  });
} else if (config.transport === "stdio") {
  transport = new StdioClientTransport({
    args: config.args,
    command: config.command,
    env: config.env ?? {},
    maxBufferSize: 524_288,
    stderr: "ignore",
  });
} else {
  transport = new StreamableHTTPClientTransport(new URL(config.url), {
    requestInit: { headers: config.headers ?? {}, redirect: "error" },
  });
}
let client = new Client({ name: "hive-connector", version: "1.0.0" });
await client.connect(transport);
const { tools } = await client.listTools();
if (config.transport === "filesystem") {
  tools.push({
    description:
      "Request local human approval for an absolute working directory. No access is granted by a remote agent confirmation. Without a local interactive terminal, approval is required and access stays denied.",
    inputSchema: {
      additionalProperties: false,
      properties: { directory: { type: "string" } },
      required: ["directory"],
      type: "object",
    },
    name: "request_directory_access",
  });
}
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
let retryDelay = 1000;
process.on("SIGINT", () => {
  running = false;
});
process.on("SIGTERM", () => {
  running = false;
});
try {
  while (running) {
    try {
      const { job } = await rpc("poll_mcp", { connection_id: connectionId });
      retryDelay = 1000;
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
        if (
          config.transport === "filesystem" &&
          claimed.request.name === "request_directory_access"
        ) {
          const { directory } = claimed.request.arguments;
          if (typeof directory !== "string") {
            throw new TypeError("directory is required");
          }
          const approved = await directories.allow(directory);
          if (approved) {
            await client.close();
            const replacement = new StdioClientTransport({
              args: [
                fileURLToPath(
                  import.meta
                    .resolve("@modelcontextprotocol/server-filesystem/dist/index.js")
                ),
                ...directories.approved,
              ],
              command: process.execPath,
              env: {},
              maxBufferSize: 524_288,
              stderr: "ignore",
            });
            client = new Client({ name: "hive-connector", version: "1.0.0" });
            await client.connect(replacement);
          }
          result = {
            content: [
              {
                text: JSON.stringify({
                  approved,
                  directory,
                  reason: approved
                    ? "approved_for_this_session"
                    : "local_human_approval_required; run connector in an interactive terminal and request again",
                }),
                type: "text",
              },
            ],
            isError: !approved,
          };
        } else {
          result = await client.callTool(claimed.request, { timeout: 25_000 });
        }
        if (!withinResultLimit(result)) {
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
    } catch (error) {
      if (error instanceof FatalHiveError) {
        throw error;
      }
      // Continue polling after transport/stale-completion failures. Never repeat a claimed tool call.
      await Bun.sleep(retryDelay);
      retryDelay = Math.min(retryDelay * 2, 10_000);
    }
  }
} finally {
  await client.close();
  if (quarantine) {
    await rm(quarantine, { force: true, recursive: true });
  }
}
