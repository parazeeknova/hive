import { readFile, realpath } from "node:fs/promises";
import path from "node:path";

import { StreamableHTTPClientTransport } from "@modelcontextprotocol/client";
import type { CallToolResult, Tool } from "@modelcontextprotocol/client";
import {
  CallToolResultSchema,
  GetPromptResultSchema,
  ListPromptsResultSchema,
  ListResourceTemplatesResultSchema,
  ListResourcesResultSchema,
  ListToolsResultSchema,
  ReadResourceResultSchema,
} from "@modelcontextprotocol/core";
import { Server } from "@modelcontextprotocol/server";
import { serveStdio } from "@modelcontextprotocol/server/stdio";

import { syncRepository } from "./code-index";
import { contains } from "./directories";
import { HiveRpc } from "./hive-rpc";
import type { HiveConfig } from "./hive-rpc";
import { LocalPreviews, previewTools } from "./local-preview";
import { FEATURES, Upstream, failure } from "./upstream";
import type { ClaimedJob, Outcome, Snapshot } from "./upstream";
import { serve } from "./worker";
import type { Service } from "./worker";

/** A bring-your-own-key upstream. URL and auth default to what Hive publishes for the slug. */
export interface ProviderConfig {
  keyFile: string;
  url?: string;
  auth?: "bearer" | "header";
  header?: string;
  readOnlyTools?: string[];
}
export interface AgentConfig {
  hive: string;
  keyFile: string;
  previews?: boolean;
  /** Shorthand for providers Hive publishes, e.g. `"context7": {"keyFile": ...}`. */
  context7?: { keyFile: string };
  tinyfish?: { keyFile: string };
  /** Any number of named providers; names become `mcp_<name>__<tool>` in Hive. */
  providers?: Record<string, ProviderConfig>;
  /** Directories whose git repositories `sync_code_index` may upload to your code index. */
  codeIndex?: { roots: string[] };
  /** Probe upstreams for MCP 2026-07-28 before using the 2025 handshake. */
  negotiate?: boolean;
}
interface LocalService extends Service {
  tools: Tool[];
  snapshot: Partial<Snapshot>;
  close: () => Promise<void>;
}
const ignore = () => null;
const VALID_NAME = /^[A-Za-z0-9_-]{1,64}$/u;

const readKey = async (keyFile: string) => {
  const raw = await readFile(keyFile, "utf-8");
  const key = raw.trim();
  if (!key || /[\r\n]/u.test(key)) {
    throw new Error("Invalid provider key file");
  }
  return key;
};
/** Merge explicit provider settings with the upstreams Hive publishes. */
export const resolveProviders = (
  config: AgentConfig,
  published: HiveConfig
) => {
  const entries: [string, ProviderConfig][] = [
    ...(["context7", "tinyfish"] as const).flatMap((name) => {
      const entry = config[name];
      return entry
        ? [[name, { keyFile: entry.keyFile }] as [string, ProviderConfig]]
        : [];
    }),
    ...Object.entries(config.providers ?? {}),
  ];
  const resolved: (Required<Omit<ProviderConfig, "url">> & {
    name: string;
    url: string;
  })[] = [];
  for (const [name, entry] of entries) {
    const known = published.integrations.find(
      (item) => item.slug === name
    )?.upstream;
    const url = entry.url ?? known?.url;
    if (!VALID_NAME.test(name) || !url || !url.startsWith("https://")) {
      console.error(
        `Hive Local: provider ${name} needs a valid name and an https url.`
      );
      continue;
    }
    resolved.push({
      auth: entry.auth ?? known?.auth ?? "bearer",
      header: entry.header ?? known?.header ?? "Authorization",
      keyFile: entry.keyFile,
      name,
      readOnlyTools: entry.readOnlyTools ?? known?.read_only_tools ?? [],
      url,
    });
  }
  return resolved;
};
const providerService = async (
  rpc: HiveRpc,
  spec: ReturnType<typeof resolveProviders>[number],
  negotiate: boolean,
  inputWait: number
): Promise<LocalService> => {
  const key = await readKey(spec.keyFile);
  const value =
    spec.auth === "bearer" && !key.startsWith("Bearer ")
      ? `Bearer ${key}`
      : key;
  const upstream = new Upstream({
    name: spec.name,
    negotiate,
    readOnlyTools: spec.readOnlyTools,
    transport: () =>
      new StreamableHTTPClientTransport(new URL(spec.url), {
        requestInit: { headers: { [spec.header]: value }, redirect: "error" },
      }),
  });
  await upstream.connect();
  const snapshot = await upstream.snapshot();
  // Operator-verified read-only tools are marked so Hive can cache them. Other
  // upstream hints are preserved; nothing is classified by name alone.
  for (const tool of snapshot.tools) {
    if (spec.readOnlyTools.includes(String(tool.name))) {
      tool.annotations = {
        ...(tool.annotations as object),
        readOnlyHint: true,
      };
    }
  }
  let lastUsed = Date.now();
  const service: LocalService = {
    close: () => upstream.close(),
    name: spec.name,
    async run(job) {
      // Check an idle session before the call. A failed ping is repaired, but a
      // failed call is never repeated: its upstream outcome may be unknown.
      if (Date.now() - lastUsed > 30_000) {
        await upstream.client
          .ping({ timeout: 5000 })
          .catch(() => upstream.repair());
      }
      lastUsed = Date.now();
      return upstream.run(rpc, service.id as string, job, inputWait);
    },
    snapshot,
    tools: snapshot.tools as unknown as Tool[],
  };
  return service;
};
const codeIndexTool: Tool = {
  annotations: {
    destructiveHint: false,
    idempotentHint: true,
    openWorldHint: false,
    readOnlyHint: false,
  },
  description:
    "Upload the committed HEAD of a local git repository to your Hive code index (only files Hive has not seen), so code_search, find_symbol, read_code and repo_overview work for it. The directory must be inside a root the user configured for Hive Local. Uncommitted changes are not uploaded.",
  inputSchema: {
    additionalProperties: false,
    properties: { directory: { type: "string" } },
    required: ["directory"],
    type: "object",
  },
  name: "sync_code_index",
};
const codeIndexService = (rpc: HiveRpc, roots: string[]): LocalService => ({
  close: () => Promise.resolve(),
  name: "code-index",
  async run(job: ClaimedJob): Promise<Outcome> {
    const directory = job.request.arguments?.directory;
    if (
      job.request.name !== "sync_code_index" ||
      typeof directory !== "string" ||
      !path.isAbsolute(directory)
    ) {
      return { result: failure as unknown as Record<string, unknown> };
    }
    const target = await realpath(directory);
    const allowed = await Promise.all(
      roots.map((root) => realpath(root).catch(() => null))
    );
    if (!allowed.some((root) => root && contains(root, target))) {
      return {
        result: {
          content: [
            {
              text: "This directory is outside the code index roots configured for Hive Local.",
              type: "text",
            },
          ],
          isError: true,
        },
      };
    }
    const synced = await syncRepository(rpc, target);
    return {
      result: {
        content: [
          {
            text: `Indexed ${synced.repo} at ${synced.commit.slice(0, 12)}: ${synced.files} files, ${synced.uploaded} uploaded, ${synced.skipped} skipped.`,
            type: "text",
          },
        ],
        structuredContent: synced as unknown as Record<string, unknown>,
      },
    };
  },
  snapshot: { tools: [codeIndexTool as unknown as Record<string, unknown>] },
  tools: [codeIndexTool],
});

export const startAgent = async (config: AgentConfig) => {
  const rpc = new HiveRpc(config.hive, await readFile(config.keyFile, "utf-8"));
  const published = await rpc.config();
  const inputWait = Math.max(1, published.connectors.poll_wait_seconds);
  const previews = new LocalPreviews(rpc);
  const services: LocalService[] = [];
  if (config.previews !== false) {
    services.push({
      close: () => {
        previews.close();
        return Promise.resolve();
      },
      name: "local-preview",
      run: async (job) => ({
        result: (await previews.call(
          job.request.name ?? "",
          job.request.arguments ?? {}
        )) as unknown as Record<string, unknown>,
      }),
      snapshot: { tools: previewTools as unknown as Record<string, unknown>[] },
      tools: previewTools,
    });
  }
  if (config.codeIndex?.roots.length) {
    services.push(codeIndexService(rpc, config.codeIndex.roots));
  }
  const providers = await Promise.allSettled(
    resolveProviders(config, published).map((spec) =>
      providerService(rpc, spec, config.negotiate === true, inputWait)
    )
  );
  for (const result of providers) {
    if (result.status === "fulfilled") {
      services.push(result.value);
    } else {
      console.error(
        "Hive Local: a provider could not connect; check its key file and url."
      );
    }
  }
  await Promise.all(
    services.map(async (service) => {
      const result = await rpc.operation("register_mcp_connection", {
        features: FEATURES,
        name: service.name,
        ...service.snapshot,
        tools: service.tools,
      });
      if (typeof result.id !== "string") {
        throw new TypeError("Hive registration failed");
      }
      service.id = result.id;
      console.error(`Hive Local: ${service.name} connected through Hive.`);
    })
  );
  let running = true;
  const stopped = serve({
    isRunning: () => running,
    pollIntervalMs: 1000,
    resultLimit: published.connectors.result_bytes,
    rpc,
    services,
    waitSeconds: published.connectors.poll_wait_seconds,
  });
  return {
    async close() {
      running = false;
      previews.close();
      await Promise.allSettled(services.map((service) => service.close()));
      await stopped.catch(ignore);
    },
    names: services.map((service) => service.name),
    rpc,
    stopped,
  };
};
const proxied = (
  agent: Awaited<ReturnType<typeof startAgent>>,
  instructions: string
) => {
  const server = new Server(
    { name: "Hive", version: "2" },
    { capabilities: { prompts: {}, resources: {}, tools: {} }, instructions }
  );
  const forward = (method: string, params: unknown) =>
    agent.rpc.request(
      method,
      (params ?? {}) as Record<string, unknown>,
      60_000
    );
  server.setRequestHandler("tools/list", async () =>
    ListToolsResultSchema.parse(await forward("tools/list", {}))
  );
  server.setRequestHandler(
    "tools/call",
    async (request) =>
      CallToolResultSchema.parse(
        await forward("tools/call", {
          arguments: request.params.arguments ?? {},
          name: request.params.name,
        })
      ) as CallToolResult
  );
  server.setRequestHandler("resources/list", async (request) =>
    ListResourcesResultSchema.parse(
      await forward("resources/list", request.params)
    )
  );
  server.setRequestHandler("resources/templates/list", async (request) =>
    ListResourceTemplatesResultSchema.parse(
      await forward("resources/templates/list", request.params)
    )
  );
  server.setRequestHandler("resources/read", async (request) =>
    ReadResourceResultSchema.parse(
      await forward("resources/read", request.params)
    )
  );
  server.setRequestHandler("prompts/list", async (request) =>
    ListPromptsResultSchema.parse(await forward("prompts/list", request.params))
  );
  server.setRequestHandler("prompts/get", async (request) =>
    GetPromptResultSchema.parse(await forward("prompts/get", request.params))
  );
  return server;
};
const main = async () => {
  const index = process.argv.indexOf("--config");
  const file = process.argv[index + 1];
  if (index === -1 || !file) {
    throw new Error(
      "Usage: bun packages/connector/src/agent.ts --config /absolute/path/agent.json"
    );
  }
  const config = JSON.parse(await readFile(file, "utf-8")) as AgentConfig;
  const agent = await startAgent(config);
  const initialized = await agent.rpc.request("initialize", {
    capabilities: {},
    clientInfo: { name: "hive-local", version: "2" },
    protocolVersion: "2025-11-25",
  });
  const handle = serveStdio(() => {
    const server = proxied(
      agent,
      typeof initialized.instructions === "string"
        ? initialized.instructions
        : "Use Hive search_tools before each new task; use call_tool with a discovered schema."
    );
    // oxlint-disable-next-line unicorn/prefer-add-event-listener -- The MCP SDK exposes only an onclose callback.
    server.onclose = () => {
      void agent.close();
    };
    return server;
  });
  const close = async () => {
    await handle.close();
    await agent.close();
  };
  process.once("SIGTERM", () => {
    void close();
  });
  process.once("SIGINT", () => {
    void close();
  });
  agent.stopped.catch(() => {
    console.error("Hive Local stopped: Hive credentials were rejected.");
    void close();
  });
};
if (import.meta.main) {
  main().catch(() => {
    console.error(
      "Hive Local could not start. Check the local configuration and key files."
    );
    process.exitCode = 1;
  });
}
