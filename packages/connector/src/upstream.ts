import { Client } from "@modelcontextprotocol/client";
import type {
  CallToolResult,
  ElicitResult,
  Progress,
  Transport,
} from "@modelcontextprotocol/client";

import type { HiveRpc } from "./hive-rpc";
import { FatalHiveError } from "./session";

/** A job claimed from Hive by `poll_mcp` or `poll_mcp_batch`. */
export interface ClaimedJob {
  id: string;
  lease: string;
  connection_id?: string;
  method?: string;
  deadline_ms?: number;
  request: {
    name?: string;
    arguments?: Record<string, unknown>;
    method?: string;
    params?: Record<string, unknown>;
  };
}
/** What a connector reports back with `complete_mcp`. */
export type Outcome =
  | { result: Record<string, unknown>; cancelled?: true }
  | { error: { code: number; message: string } };

export interface Snapshot {
  tools: Record<string, unknown>[];
  resources: Record<string, unknown>[];
  resource_templates: Record<string, unknown>[];
  prompts: Record<string, unknown>[];
}

const ignore = () => null;
export const FEATURES = ["bridge", "input", "progress", "cancel", "long_poll"];
export const failure: CallToolResult = {
  content: [
    {
      text: "Upstream MCP call failed or exceeded limits. Execution outcome may be unknown; do not retry mutations automatically.",
      type: "text",
    },
  ],
  isError: true,
};
const cancelledResult: CallToolResult = {
  content: [{ text: "Cancelled at the caller's request.", type: "text" }],
  isError: true,
};

/** The job a client is executing, so server-to-client requests reach the right caller. */
interface Running {
  rpc: HiveRpc;
  connectionId: string;
  job: ClaimedJob;
  controller: AbortController;
  deadline: number;
}

const HEARTBEAT_MS = 10_000;
const PROGRESS_INTERVAL_MS = 1000;
const IDLE_TIMEOUT_MS = 120_000;
/** Hard ceiling on waiting for a human to answer an elicitation. */
const INPUT_WAIT_MS = 600_000;

/** Ask Hive whether the caller cancelled; tolerate servers without the operation. */
const heartbeat = async (
  running: Running,
  message?: string,
  progress?: { progress: number; total?: number }
) => {
  try {
    const status = await running.rpc.operation("progress_mcp", {
      connection_id: running.connectionId,
      job_id: running.job.id,
      lease: running.job.lease,
      ...(message ? { message } : {}),
      ...progress,
    });
    if (status.cancel_requested === true) {
      running.controller.abort();
    }
  } catch (error) {
    if (error instanceof FatalHiveError) {
      running.controller.abort();
    }
  }
};

/**
 * Relay an upstream `elicitation/create` to the Hive caller and wait for the
 * user's answer. Hive declines on behalf of callers that cannot show it.
 */
const relayElicitation = async (
  running: Running | null,
  params: Record<string, unknown>,
  wait: number
): Promise<ElicitResult> => {
  if (!running) {
    return { action: "decline" };
  }
  const identity = {
    connection_id: running.connectionId,
    job_id: running.job.id,
    lease: running.job.lease,
  };
  try {
    const asked = await running.rpc.operation("elicit_mcp", {
      ...identity,
      requests: { input: { method: "elicitation/create", params } },
    });
    if (asked.accepted !== true) {
      return { action: "cancel" };
    }
    const until = Math.min(running.deadline, Date.now() + INPUT_WAIT_MS);
    while (Date.now() < until && !running.controller.signal.aborted) {
      // oxlint-disable-next-line no-await-in-loop -- Each long-poll waits for the user's answer.
      const answer = await running.rpc.operation(
        "await_input_mcp",
        { ...identity, wait_seconds: wait },
        (wait + 15) * 1000
      );
      if (answer.cancelled === true) {
        running.controller.abort();
        return { action: "cancel" };
      }
      const responses = answer.responses as
        | Record<string, ElicitResult>
        | undefined;
      if (responses?.input) {
        return responses.input;
      }
    }
  } catch {
    // Older Hive servers cannot relay input; the upstream sees a decline.
    return { action: "decline" };
  }
  return { action: "cancel" };
};

export interface UpstreamOptions {
  name: string;
  transport: () => Transport;
  /** Probe for the 2026-07-28 protocol before falling back to 2025 initialize. */
  negotiate?: boolean;
  /** Explicitly verified by the operator, never inferred from an upstream hint. */
  readOnlyTools?: readonly string[];
  /** Called when the upstream reports that its tools, resources or prompts changed. */
  onChanged?: () => void;
}

/**
 * One upstream MCP server behind a connector. Server-to-client elicitation is
 * relayed to the Hive caller, progress and cancellation flow both ways, and a
 * failed call heals the session. Only explicitly verified read-only tools may
 * be retried once; mutations and unknown operations are never repeated.
 */
export class Upstream {
  client: Client;
  private running: Running | null = null;
  private readonly options: UpstreamOptions;
  private inputWait = 20;
  constructor(options: UpstreamOptions) {
    this.options = options;
    this.client = this.create();
  }
  private create() {
    const changed = { onChanged: () => this.options.onChanged?.() };
    const client = new Client(
      { name: "hive-connector", version: "2.0.0" },
      {
        capabilities: { elicitation: { form: {}, url: {} } },
        listChanged: { prompts: changed, resources: changed, tools: changed },
        ...(this.options.negotiate
          ? { versionNegotiation: { mode: "auto" as const } }
          : {}),
      }
    );
    client.setRequestHandler("elicitation/create", (request) =>
      relayElicitation(
        this.running,
        request.params as Record<string, unknown>,
        this.inputWait
      )
    );
    return client;
  }
  async connect() {
    await this.client.connect(this.options.transport());
  }
  /** Replace the session after a failure; retry decisions belong to run(). */
  async repair() {
    await this.client.close().catch(ignore);
    this.client = this.create();
    await this.connect();
  }
  close() {
    return this.client.close();
  }
  /** Everything the upstream advertises, for `register_mcp_connection`. */
  async snapshot(): Promise<Snapshot> {
    const capabilities = this.client.getServerCapabilities() ?? {};
    const { tools } = await this.client.listTools();
    const resources = capabilities.resources
      ? await this.client.listResources().then(
          (r) => r.resources,
          () => []
        )
      : [];
    const templates = capabilities.resources
      ? await this.client.listResourceTemplates().then(
          (r) => r.resourceTemplates,
          () => []
        )
      : [];
    const prompts = capabilities.prompts
      ? await this.client.listPrompts().then(
          (r) => r.prompts,
          () => []
        )
      : [];
    return {
      prompts: prompts as unknown as Record<string, unknown>[],
      resource_templates: templates as unknown as Record<string, unknown>[],
      resources: resources as unknown as Record<string, unknown>[],
      tools: tools as unknown as Record<string, unknown>[],
    };
  }
  private isReadOnlyTool(method: string, job: ClaimedJob) {
    return (
      method === "tools/call" &&
      this.options.readOnlyTools?.includes(job.request.name ?? "") === true
    );
  }
  /** Execute one claimed job against the upstream. */
  async run(
    rpc: HiveRpc,
    connectionId: string,
    job: ClaimedJob,
    inputWait: number
  ): Promise<Outcome> {
    const controller = new AbortController();
    const deadline = job.deadline_ms ?? Date.now() + 30_000;
    const running: Running = { connectionId, controller, deadline, job, rpc };
    this.running = running;
    this.inputWait = inputWait;
    let lastProgress = 0;
    const beat = setInterval(() => {
      void heartbeat(running);
    }, HEARTBEAT_MS);
    const options = {
      maxTotalTimeout: Math.max(1000, deadline - Date.now() - 2000),
      onprogress: (progress: Progress) => {
        if (Date.now() - lastProgress < PROGRESS_INTERVAL_MS) {
          return;
        }
        lastProgress = Date.now();
        void heartbeat(
          running,
          progress.message ?? "Working",
          progress.total === undefined
            ? { progress: progress.progress }
            : { progress: progress.progress, total: progress.total }
        );
      },
      resetTimeoutOnProgress: true,
      signal: controller.signal,
      timeout: IDLE_TIMEOUT_MS,
    };
    const method = job.method ?? job.request.method ?? "tools/call";
    try {
      if (method === "resources/read") {
        const result = await this.client.readResource(
          job.request.params as { uri: string },
          options
        );
        return { result: result as unknown as Record<string, unknown> };
      }
      if (method === "prompts/get") {
        const result = await this.client.getPrompt(
          job.request.params as {
            name: string;
            arguments?: Record<string, string>;
          },
          options
        );
        return { result: result as unknown as Record<string, unknown> };
      }
      const result = await this.client.callTool(
        {
          arguments: job.request.arguments ?? {},
          name: job.request.name ?? "",
        },
        options
      );
      return { result: result as unknown as Record<string, unknown> };
    } catch (error) {
      if (error instanceof FatalHiveError) {
        throw error;
      }
      if (controller.signal.aborted) {
        return {
          cancelled: true,
          result: cancelledResult as unknown as Record<string, unknown>,
        };
      }
      const retryReadOnly = this.isReadOnlyTool(method, job);
      // Do not print exception messages: upstream failures may contain credentials
      // or document contents. These fields are sufficient to correlate the fault.
      console.error(
        JSON.stringify({
          connection: this.options.name,
          event: "upstream_request_failed",
          job_id: job.id,
          method,
          retry_read_only: retryReadOnly,
        })
      );
      const repaired = await this.repair().then(
        () => true,
        () => false
      );
      if (
        retryReadOnly &&
        repaired &&
        !controller.signal.aborted &&
        Date.now() < deadline - 2000
      ) {
        try {
          const result = await this.client.callTool(
            {
              arguments: job.request.arguments ?? {},
              name: job.request.name ?? "",
            },
            {
              ...options,
              maxTotalTimeout: Math.max(1, deadline - Date.now() - 2000),
            }
          );
          console.error(
            JSON.stringify({
              connection: this.options.name,
              event: "upstream_read_recovered",
              job_id: job.id,
            })
          );
          return { result: result as unknown as Record<string, unknown> };
        } catch {
          // One recovery attempt only; never replay application-level tool errors.
        }
      }
      if (controller.signal.aborted) {
        return {
          cancelled: true,
          result: cancelledResult as unknown as Record<string, unknown>,
        };
      }
      if (method === "tools/call") {
        return { result: failure as unknown as Record<string, unknown> };
      }
      return {
        error: {
          code: -32_603,
          message: "Upstream request failed; execution outcome may be unknown",
        },
      };
    } finally {
      clearInterval(beat);
      this.running = null;
    }
  }
}
