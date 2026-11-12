/* oxlint-disable no-await-in-loop, no-unmodified-loop-condition -- A single multiplexed poller bounds all local connections to one request per second; close() stops it. */
import { readFile } from "node:fs/promises";

import {
  Client,
  StreamableHTTPClientTransport,
} from "@modelcontextprotocol/client";
import type { CallToolResult, Tool } from "@modelcontextprotocol/client";
import {
  CallToolResultSchema,
  ListToolsResultSchema,
} from "@modelcontextprotocol/core";
import { Server } from "@modelcontextprotocol/server";
import { serveStdio } from "@modelcontextprotocol/server/stdio";

import { HiveRpc } from "./hive-rpc";
import { withinResultLimit } from "./limits";
import { LocalPreviews, previewTools } from "./local-preview";
import { FatalHiveError } from "./session";

type ProviderName = "context7" | "tinyfish";
export interface AgentConfig {
  hive: string;
  keyFile: string;
  previews?: boolean;
  context7?: { keyFile: string };
  tinyfish?: { keyFile: string };
}
interface Service {
  name: string;
  id?: string;
  tools: Tool[];
  call: (
    name: string,
    args: Record<string, unknown>
  ) => Promise<CallToolResult>;
  close: () => Promise<void>;
}
const ignore = () => null;
const READ_ONLY_CONTEXT7 = new Set(["resolve-library-id", "query-docs"]);
const provider = async (
  name: ProviderName,
  keyFile: string
): Promise<Service> => {
  const raw = await readFile(keyFile, "utf-8");
  const key = raw.trim();
  if (!key || /[\r\n]/u.test(key)) {
    throw new Error("Invalid provider key file");
  }
  const url =
    name === "context7"
      ? "https://mcp.context7.com/mcp"
      : "https://agent.tinyfish.ai/mcp";
  const headers =
    name === "context7"
      ? { Authorization: key.startsWith("Bearer ") ? key : `Bearer ${key}` }
      : { "X-API-Key": key };
  let client = new Client({ name: "hive-local", version: "1" });
  const connect = async () => {
    await client.connect(
      new StreamableHTTPClientTransport(new URL(url), {
        requestInit: { headers, redirect: "error" },
      })
    );
  };
  await connect();
  const { tools } = await client.listTools();
  // These two Context7 operations only read documentation. Other upstream hints
  // are preserved; no browsing/mutation operation is classified by name alone.
  if (name === "context7") {
    for (const tool of tools) {
      if (READ_ONLY_CONTEXT7.has(tool.name)) {
        tool.annotations = { ...tool.annotations, readOnlyHint: true };
      }
    }
  }
  let lastUsed = Date.now();
  const repair = async () => {
    await client.close().catch(ignore);
    client = new Client({ name: "hive-local", version: "1" });
    await connect();
  };
  return {
    async call(tool, args) {
      // Check an idle session before the call. A failed ping is repaired, but a
      // failed call is never repeated: its upstream outcome may be unknown.
      if (Date.now() - lastUsed > 30_000) {
        try {
          await client.ping({ timeout: 5000 });
        } catch {
          await repair();
        }
      }
      lastUsed = Date.now();
      try {
        return CallToolResultSchema.parse(
          await client.callTool(
            { arguments: args, name: tool },
            { timeout: 25_000 }
          )
        );
      } catch {
        await repair().catch(ignore);
        throw new Error(
          "Upstream call failed; execution outcome may be unknown. No automatic retry was made."
        );
      }
    },
    close: () => client.close(),
    name,
    tools,
  };
};
const failure: CallToolResult = {
  content: [
    {
      text: "Local/upstream tool failed or exceeded limits. Execution outcome may be unknown; no automatic retry was made.",
      type: "text",
    },
  ],
  isError: true,
};
const connectProviders = async (config: AgentConfig) => {
  const wanted = (["context7", "tinyfish"] as const).flatMap((name) => {
    const entry = config[name];
    return entry ? [{ keyFile: entry.keyFile, name }] : [];
  });
  const results = await Promise.allSettled(
    wanted.map(({ name, keyFile }) => provider(name, keyFile))
  );
  const services: Service[] = [];
  for (const [index, result] of results.entries()) {
    if (result.status === "fulfilled") {
      services.push(result.value);
    } else {
      console.error(
        `Hive Local: ${wanted[index]?.name} could not connect; check its key file.`
      );
    }
  }
  return services;
};
export const startAgent = async (config: AgentConfig) => {
  const rpc = new HiveRpc(config.hive, await readFile(config.keyFile, "utf-8"));
  const previews = new LocalPreviews(rpc);
  const services: Service[] =
    config.previews === false
      ? []
      : [
          {
            call: (name, args) => previews.call(name, args),
            close: () => {
              previews.close();
              return Promise.resolve();
            },
            name: "local-preview",
            tools: previewTools,
          },
        ];
  services.push(...(await connectProviders(config)));
  for (const service of services) {
    const result = await rpc.operation("register_mcp_connection", {
      name: service.name,
      tools: service.tools,
    });
    if (typeof result.id !== "string") {
      throw new TypeError("Hive registration failed");
    }
    service.id = result.id;
    console.error(`Hive Local: ${service.name} connected through Hive.`);
  }
  let running = true;
  const busy = new Set<string>();
  const active = new Set<Promise<void>>();
  const execute = async (service: Service, job: Record<string, unknown>) => {
    let result: CallToolResult;
    try {
      const request = job.request as {
        name: string;
        arguments: Record<string, unknown>;
      };
      if (!service.tools.some((tool) => tool.name === request.name)) {
        throw new Error("Tool is not registered locally");
      }
      result = await service.call(request.name, request.arguments);
      if (!withinResultLimit(result)) {
        throw new Error("Upstream result exceeds Hive's 512 KiB limit");
      }
    } catch {
      result = failure;
    }
    // Exactly one completion attempt. Never execute a claimed job twice.
    await rpc.operation("complete_mcp", {
      connection_id: service.id,
      job_id: job.id,
      lease: job.lease,
      result,
    });
  };
  const dispatch = (available: Service[], job: Record<string, unknown>) => {
    const service = available.find((item) => item.id === job.connection_id);
    if (!service?.id) {
      return;
    }
    const { id } = service;
    busy.add(id);
    const task = execute(service, job)
      .catch(() => {
        console.error(
          "Hive Local: completion was not acknowledged; the claimed call will not be repeated."
        );
      })
      .finally(() => {
        busy.delete(id);
        active.delete(task);
      });
    active.add(task);
  };
  const loop = async () => {
    let delay = 1000;
    while (running) {
      const available = services.filter(
        (service) => service.id && !busy.has(service.id)
      );
      if (available.length > 0) {
        try {
          // One request claims at most one job across every idle local service,
          // so adding providers does not multiply the polling rate.
          const polled = await rpc.operation("poll_mcp_batch", {
            connection_ids: available.map((service) => service.id),
          });
          const { job } = polled;
          if (job && typeof job === "object" && !Array.isArray(job)) {
            dispatch(available, job as Record<string, unknown>);
          }
          delay = 1000;
        } catch (error) {
          if (error instanceof FatalHiveError) {
            throw error;
          }
          delay = Math.min(10_000, delay * 2);
        }
      }
      await Bun.sleep(delay);
    }
  };
  const stopped = loop();
  return {
    async close() {
      running = false;
      previews.close();
      await Promise.allSettled(services.map((service) => service.close()));
      await Promise.allSettled(active);
      await stopped.catch(ignore);
    },
    names: services.map((service) => service.name),
    rpc,
    stopped,
  };
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
    clientInfo: { name: "hive-local", version: "1" },
    protocolVersion: "2025-11-25",
  });
  const handle = serveStdio(() => {
    const server = new Server(
      { name: "Hive", version: "1" },
      {
        capabilities: { tools: {} },
        instructions:
          typeof initialized.instructions === "string"
            ? initialized.instructions
            : "Use Hive search_tools before each new task; use call_tool with a discovered schema.",
      }
    );
    server.setRequestHandler("tools/list", async () =>
      ListToolsResultSchema.parse(await agent.rpc.request("tools/list", {}))
    );
    server.setRequestHandler("tools/call", (request) =>
      agent.rpc.tool(request.params.name, request.params.arguments ?? {})
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
