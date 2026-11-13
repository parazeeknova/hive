import { CallToolResultSchema } from "@modelcontextprotocol/core";

import { FatalHiveError } from "./session";

/** Limits and providers a Hive deployment publishes at `/api/config`. */
export interface HiveConfig {
  connectors: {
    argument_bytes: number;
    result_bytes: number;
    poll_wait_seconds: number;
    job_ttl_seconds: number;
    input_requests_per_job: number;
    code_file_bytes: number;
    code_manifest_files_per_call: number;
    code_blobs_per_upload: number;
  };
  limits: { request_body_bytes: number };
  integrations: {
    slug: string;
    name: string;
    byok: boolean;
    upstream: {
      url: string;
      auth: "bearer" | "header";
      header: string;
      read_only_tools: string[];
    } | null;
  }[];
}

/** Defaults for Hive servers that predate `/api/config` connector limits. */
const FALLBACK: HiveConfig = {
  connectors: {
    argument_bytes: 131_072,
    code_blobs_per_upload: 64,
    code_file_bytes: 262_144,
    code_manifest_files_per_call: 2000,
    input_requests_per_job: 4,
    job_ttl_seconds: 35,
    poll_wait_seconds: 0,
    result_bytes: 524_288,
  },
  integrations: [],
  limits: { request_body_bytes: 1_048_576 },
};
const RESPONSE_LIMIT = 2_097_152;
/** Connector operations whose rejection means the connection itself is gone. */
const FATAL_OPERATION_ERRORS = new Set(["forbidden", "invalid_connection_id"]);

/** Read a deployment's public configuration; older servers get conservative defaults. */
export const loadConfig = async (hive: string | URL): Promise<HiveConfig> => {
  try {
    const response = await fetch(new URL("/api/config", hive), {
      headers: { Accept: "application/json" },
      redirect: "error",
      signal: AbortSignal.timeout(10_000),
    });
    const body = (await response.json()) as Partial<HiveConfig>;
    return {
      connectors: { ...FALLBACK.connectors, ...body.connectors },
      integrations: body.integrations ?? [],
      limits: { ...FALLBACK.limits, ...body.limits },
    };
  } catch {
    return FALLBACK;
  }
};
export const assertHiveUrl = (url: URL) => {
  if (
    url.protocol !== "https:" &&
    !(
      url.protocol === "http:" &&
      ["localhost", "127.0.0.1"].includes(url.hostname)
    )
  ) {
    throw new Error("Hive requires HTTPS outside loopback");
  }
  if (url.username || url.password) {
    throw new Error("Hive URL cannot contain credentials");
  }
};

export class HiveRpc {
  readonly url: URL;
  private readonly key: string;
  private cached: HiveConfig | undefined;
  constructor(url: string, key: string) {
    this.url = new URL(url);
    assertHiveUrl(this.url);
    this.key = key.trim();
    if (!this.key) {
      throw new Error("Hive key file is empty");
    }
  }
  /** Read and cache the deployment's public configuration. */
  async config(): Promise<HiveConfig> {
    this.cached ??= await loadConfig(this.url);
    return this.cached;
  }
  async request(
    method: string,
    params: Record<string, unknown>,
    timeoutMs = 40_000
  ) {
    const response = await fetch(this.url, {
      body: JSON.stringify({
        id: crypto.randomUUID(),
        jsonrpc: "2.0",
        method,
        params,
      }),
      headers: {
        Accept: "application/json, text/event-stream",
        Authorization: `Bearer ${this.key}`,
        "Content-Type": "application/json",
        "MCP-Protocol-Version": "2025-11-25",
      },
      method: "POST",
      redirect: "error",
      signal: AbortSignal.timeout(timeoutMs),
    });
    if ([401, 403].includes(response.status)) {
      throw new FatalHiveError("Hive credentials rejected");
    }
    if (!response.ok || !response.body) {
      throw new Error(`Hive request failed (${response.status})`);
    }
    const reader = response.body.getReader();
    const chunks: Uint8Array[] = [];
    let bytes = 0;
    try {
      while (true) {
        // oxlint-disable-next-line no-await-in-loop -- Stream chunks must be read in order to enforce the size cap.
        const { done, value } = await reader.read();
        if (done) {
          break;
        }
        bytes += value.length;
        if (bytes > RESPONSE_LIMIT) {
          throw new Error("Hive response exceeds the local agent limit");
        }
        chunks.push(value);
      }
    } finally {
      await reader.cancel();
    }
    const value = JSON.parse(Buffer.concat(chunks).toString("utf-8")) as {
      error?: unknown;
      result?: Record<string, unknown>;
    };
    if (value.error || !value.result) {
      throw new Error("Hive protocol request failed");
    }
    return value.result;
  }
  async tool(name: string, args: Record<string, unknown>, timeoutMs?: number) {
    return CallToolResultSchema.parse(
      await this.request("tools/call", { arguments: args, name }, timeoutMs)
    );
  }
  /** Call a Hive tool that returns JSON, failing on tool errors. */
  async operation(
    name: string,
    args: Record<string, unknown>,
    timeoutMs?: number
  ) {
    const result = await this.tool(name, args, timeoutMs);
    if (result.isError || !result.structuredContent) {
      const code = String(
        (result.structuredContent as { error?: unknown } | undefined)?.error
      );
      if (FATAL_OPERATION_ERRORS.has(code)) {
        throw new FatalHiveError("Hive connection is unavailable or revoked");
      }
      throw new Error(`Hive operation ${name} failed (${code})`);
    }
    return result.structuredContent as Record<string, unknown>;
  }
}
