import { CallToolResultSchema } from "@modelcontextprotocol/core";

import { FatalHiveError } from "./session";

export class HiveRpc {
  readonly url: URL;
  private readonly key: string;
  constructor(url: string, key: string) {
    this.url = new URL(url);
    if (
      this.url.protocol !== "https:" &&
      !(
        this.url.protocol === "http:" &&
        ["localhost", "127.0.0.1"].includes(this.url.hostname)
      )
    ) {
      throw new Error("Hive requires HTTPS outside loopback");
    }
    if (this.url.username || this.url.password) {
      throw new Error("Hive URL cannot contain credentials");
    }
    this.key = key.trim();
    if (!this.key) {
      throw new Error("Hive key file is empty");
    }
  }
  async request(method: string, params: Record<string, unknown>) {
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
      signal: AbortSignal.timeout(40_000),
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
        if (bytes > 2_097_152) {
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
  async tool(name: string, args: Record<string, unknown>) {
    return CallToolResultSchema.parse(
      await this.request("tools/call", { arguments: args, name })
    );
  }
  async operation(name: string, args: Record<string, unknown>) {
    const result = await this.tool(name, args);
    if (result.isError || !result.structuredContent) {
      throw new Error("Hive operation failed");
    }
    return result.structuredContent as Record<string, unknown>;
  }
}
