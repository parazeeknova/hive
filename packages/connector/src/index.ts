import { mkdtemp, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { StreamableHTTPClientTransport } from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

import { Directories } from "./directories";
import { HiveRpc } from "./hive-rpc";
import { FEATURES, Upstream } from "./upstream";
import type { ClaimedJob, Outcome } from "./upstream";
import { serve } from "./worker";

type Config = {
  name: string;
  hive: string;
  keyFile: string;
  connectionId?: string;
  workingDirectory?: string;
  /** Probe upstreams for MCP 2026-07-28 before using the 2025 handshake. */
  negotiate?: boolean;
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
const rpc = new HiveRpc(config.hive, await readFile(config.keyFile, "utf-8"));
const settings = await rpc.config();
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
const filesystemServer = () =>
  fileURLToPath(
    import.meta.resolve("@modelcontextprotocol/server-filesystem/dist/index.js")
  );
const buildTransport = () => {
  if (config.transport === "filesystem") {
    return new StdioClientTransport({
      args: [
        filesystemServer(),
        ...directories.approved,
        ...(quarantine ? [quarantine] : []),
      ],
      command: process.execPath,
      env: {},
      maxBufferSize: settings.connectors.result_bytes,
      stderr: "ignore",
    });
  }
  if (config.transport === "stdio") {
    return new StdioClientTransport({
      args: config.args,
      command: config.command,
      env: config.env ?? {},
      maxBufferSize: settings.connectors.result_bytes,
      stderr: "ignore",
    });
  }
  return new StreamableHTTPClientTransport(new URL(config.url), {
    requestInit: { headers: config.headers ?? {}, redirect: "error" },
  });
};
let refresh = () => {
  // Replaced once the connection is registered.
};
const upstream = new Upstream({
  name: config.name,
  negotiate: config.negotiate === true,
  onChanged: () => refresh(),
  transport: buildTransport,
});
await upstream.connect();
const directoryTool = {
  description:
    "Request local human approval for an absolute working directory. No access is granted by a remote agent confirmation. Without a local interactive terminal, approval is required and access stays denied.",
  inputSchema: {
    additionalProperties: false,
    properties: { directory: { type: "string" } },
    required: ["directory"],
    type: "object",
  },
  name: "request_directory_access",
};
const snapshot = async () => {
  const current = await upstream.snapshot();
  if (config.transport === "filesystem") {
    current.tools.push(directoryTool);
  }
  return current;
};
let advertised = await snapshot();
const register = () =>
  rpc.operation("register_mcp_connection", {
    features: FEATURES,
    name: config.name,
    ...advertised,
  });
let { connectionId } = config;
if (!connectionId) {
  const registered = await register();
  if (typeof registered.id !== "string") {
    throw new TypeError("Missing connection ID");
  }
  connectionId = registered.id;
}
const id = connectionId;
console.log(
  `Connected ${config.name}: ${id}. Save connectionId in your config to reuse it.`
);
// Upstream list changes refresh the snapshot so Hive clients see new tools at once.
const refreshSnapshot = async () => {
  try {
    advertised = await snapshot();
    await register();
  } catch {
    console.error(
      "Hive connector: snapshot refresh failed; keeping the previous one."
    );
  }
};
refresh = () => {
  void refreshSnapshot();
};
const approveDirectory = async (job: ClaimedJob): Promise<Outcome> => {
  const directory = job.request.arguments?.directory;
  if (typeof directory !== "string") {
    throw new TypeError("directory is required");
  }
  const approved = await directories.allow(directory);
  if (approved) {
    await upstream.repair();
  }
  return {
    result: {
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
    },
  };
};
const allowed = () =>
  new Set(advertised.tools.map((tool) => String(tool.name)));
let running = true;
process.on("SIGINT", () => {
  running = false;
});
process.on("SIGTERM", () => {
  running = false;
});
try {
  await serve({
    isRunning: () => running,
    pollIntervalMs: Math.min(
      10_000,
      Math.max(100, Number(process.env.HIVE_POLL_MS ?? 1000) || 1000)
    ),
    resultLimit: settings.connectors.result_bytes,
    rpc,
    services: [
      {
        id,
        name: config.name,
        run: (job) => {
          const method = job.method ?? "tools/call";
          if (method === "tools/call") {
            if (!allowed().has(job.request.name ?? "")) {
              throw new Error("Tool not in local allowlist");
            }
            if (
              config.transport === "filesystem" &&
              job.request.name === "request_directory_access"
            ) {
              return approveDirectory(job);
            }
          }
          return upstream.run(
            rpc,
            id,
            job,
            Math.max(1, settings.connectors.poll_wait_seconds)
          );
        },
      },
    ],
    waitSeconds: settings.connectors.poll_wait_seconds,
  });
} finally {
  await upstream.close();
  if (quarantine) {
    await rm(quarantine, { force: true, recursive: true });
  }
}
