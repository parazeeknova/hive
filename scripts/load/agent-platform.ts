/**
 * Heavy-traffic checks for Hiveway's agent platform against a local stack.
 *
 *   bun scripts/load/agent-platform.ts
 *
 * Requires a running Hiveway whose PostgreSQL is reachable at LOAD_DATABASE_URL
 * (accounts are seeded directly), and HIVE_PROVIDER_URL_CONTEXT7 pointing at
 * this script's fake provider (http://127.0.0.1:47100/mcp by default). Failure
 * scenarios pause the Redis container named LOAD_REDIS_CONTAINER and terminate
 * Hiveway's LISTEN connection. Never point this at shared infrastructure.
 */
import { SQL } from "bun";

const base = (process.env.HIVE_URL ?? "http://127.0.0.1:3101").replace(
  /\/$/u,
  ""
);
const databaseUrl =
  process.env.LOAD_DATABASE_URL ??
  "postgres://hive:hive-test@127.0.0.1:55432/hive_load";
const redisContainer = process.env.LOAD_REDIS_CONTAINER ?? "hive-load-redis";
const upstreamPort = Number(process.env.LOAD_UPSTREAM_PORT ?? 47_100);
const scale = Number(process.env.LOAD_SCALE ?? 1);
const only = process.env.LOAD_ONLY?.split(",").filter(Boolean);

// --- Reporting ---------------------------------------------------------------

interface Sample {
  status: number;
  ms: number;
}
const failures: string[] = [];
const check = (scenario: string, ok: boolean, message: string) => {
  console.log(`  ${ok ? "✓" : "✗"} ${message}`);
  if (!ok) {
    failures.push(`${scenario}: ${message}`);
  }
};
const percentile = (values: number[], p: number) => {
  if (!values.length) {
    return 0;
  }
  const sorted = values.toSorted((a, b) => a - b);
  return (
    sorted[
      Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length))
    ] ?? 0
  );
};
const summarize = (samples: Sample[], seconds: number) => {
  const statuses = new Map<number, number>();
  for (const sample of samples) {
    statuses.set(sample.status, (statuses.get(sample.status) ?? 0) + 1);
  }
  const ms = samples.map((s) => s.ms);
  console.log(
    `  ${samples.length} requests in ${seconds.toFixed(1)}s (${(samples.length / seconds).toFixed(0)}/s) · statuses ${JSON.stringify(Object.fromEntries(statuses))} · p50 ${percentile(ms, 50).toFixed(0)}ms p95 ${percentile(ms, 95).toFixed(0)}ms max ${percentile(ms, 100).toFixed(0)}ms`
  );
  return statuses;
};
const timed = async <T>(work: () => Promise<T>) => {
  const started = performance.now();
  const value = await work();
  return { seconds: (performance.now() - started) / 1000, value };
};
/** Run `tasks` with at most `limit` in flight. */
const pool = async <T>(limit: number, tasks: (() => Promise<T>)[]) => {
  const results: T[] = Array.from({ length: tasks.length });
  let next = 0;
  const lane = async () => {
    while (next < tasks.length) {
      const index = next;
      next += 1;
      // oxlint-disable-next-line no-await-in-loop -- Each lane runs its share of tasks in order.
      results[index] = await (tasks[index] as () => Promise<T>)();
    }
  };
  await Promise.all(
    Array.from({ length: Math.min(limit, tasks.length) }, lane)
  );
  return results;
};

// --- HTTP ----------------------------------------------------------------------

// oxlint-disable-next-line typescript/no-explicit-any -- Responses are untyped JSON, probed defensively.
type Json = any;
interface Reply {
  status: number;
  ms: number;
  body: Json;
  retryAfter: string | null;
}
const request = async (
  path: string,
  token: string,
  init: {
    method?: string;
    body?: unknown;
    headers?: Record<string, string>;
  } = {}
): Promise<Reply> => {
  const started = performance.now();
  try {
    const response = await fetch(`${base}${path}`, {
      ...(init.body === undefined ? {} : { body: JSON.stringify(init.body) }),
      headers: {
        "content-type": "application/json",
        ...(token ? { authorization: `Bearer ${token}` } : {}),
        ...init.headers,
      },
      method: init.method ?? (init.body === undefined ? "GET" : "POST"),
      signal: AbortSignal.timeout(120_000),
    });
    const text = await response.text();
    let body: Json = text;
    try {
      body = text ? JSON.parse(text) : null;
    } catch {
      // Non-JSON error bodies stay as text.
    }
    return {
      body,
      ms: performance.now() - started,
      retryAfter: response.headers.get("retry-after"),
      status: response.status,
    };
  } catch (error) {
    return {
      body: String(error),
      ms: performance.now() - started,
      retryAfter: null,
      status: 0,
    };
  }
};
let rpcId = 0;
const rpc = (
  token: string,
  method: string,
  params: unknown = {},
  headers: Record<string, string> = {}
) => {
  rpcId += 1;
  return request("/mcp", token, {
    body: { id: rpcId, jsonrpc: "2.0", method, params },
    headers,
  });
};
const call = async (token: string, name: string, args: unknown) => {
  const reply = await rpc(token, "tools/call", { arguments: args, name });
  return { ...reply, result: reply.body?.result };
};
const textOf = (result: Json): string => result?.content?.[0]?.text ?? "";

// --- Accounts ------------------------------------------------------------------

const db = new SQL(databaseUrl);
const hex = (bytes: Uint8Array) => Buffer.from(bytes).toString("hex");
interface Account {
  wallet: string;
  session: string;
  key: string;
}
const runId = crypto.randomUUID().slice(0, 8);
let accountCount = 0;
/** A GitHub-style browser session (seeded) and an agent API key issued through the API. */
const account = async (): Promise<Account> => {
  accountCount += 1;
  const wallet = `github:load-${runId}-${accountCount}`;
  const session = `hwa_${hex(crypto.getRandomValues(new Uint8Array(32)))}`;
  const hash = new Bun.CryptoHasher("sha256").update(session).digest("hex");
  await db`INSERT INTO agent_credentials (id, wallet, name, token_hash, scopes, per_call_limit, total_limit, reserved, expires_at, created_at)
    VALUES (${crypto.randomUUID()}, ${wallet}, 'load-session', decode(${hash}, 'hex'), '{capabilities:read,receipts:read,capabilities:invoke}'::text[], 100, 100, 0, ${Math.floor(Date.now() / 1000) + 7200}, 0)`;
  const issued = await request("/v1/account/credentials", session, {
    body: { expires_in: 86_400, name: "load agent" },
  });
  if (issued.status !== 200 || typeof issued.body?.token !== "string") {
    throw new Error(
      `issuing an API key failed: ${issued.status} ${JSON.stringify(issued.body)}`
    );
  }
  return { key: issued.body.token, session, wallet };
};
/** The scenario's only account. */
const only1 = (group: Account[]) => {
  const [first] = group;
  if (!first) {
    throw new Error("scenario needs an account");
  }
  return first;
};
const accounts = (count: number) =>
  Promise.all(Array.from({ length: count }, account));

// --- Fake provider -------------------------------------------------------------

/** Executions per unique query, to prove no call ran twice. */
const executions = new Map<string, number>();
let providerRequests = 0;
const sessions = new Set<string>();
const sse = (messages: unknown[]) =>
  new Response(
    messages
      .map((m) => `event: message\ndata: ${JSON.stringify(m)}\n\n`)
      .join(""),
    {
      headers: { "content-type": "text/event-stream" },
    }
  );
const provider = Bun.serve({
  async fetch(req) {
    providerRequests += 1;
    if (!req.headers.get("authorization")?.startsWith("Bearer load-key-")) {
      return new Response(null, { status: 401 });
    }
    const message = (await req.json()) as Json;
    if (message.method === "initialize") {
      const id = crypto.randomUUID();
      sessions.add(id);
      return Response.json(
        {
          id: message.id,
          jsonrpc: "2.0",
          result: {
            capabilities: { tools: {} },
            protocolVersion: "2025-11-25",
            serverInfo: { name: "load-provider", version: "1" },
          },
        },
        { headers: { "mcp-session-id": id } }
      );
    }
    const session = req.headers.get("mcp-session-id") ?? "";
    // Expire one in forty sessions before executing anything.
    if (
      !sessions.has(session) ||
      (message.method === "tools/call" && Math.random() < 0.025)
    ) {
      sessions.delete(session);
      return new Response(null, { status: 404 });
    }
    if (message.method === undefined || message.id === undefined) {
      return new Response(null, { status: 202 });
    }
    if (message.method === "tools/list") {
      return Response.json({
        id: message.id,
        jsonrpc: "2.0",
        result: {
          tools: [
            { inputSchema: { type: "object" }, name: "resolve-library-id" },
            { inputSchema: { type: "object" }, name: "query-docs" },
          ],
        },
      });
    }
    const q = String(message.params?.arguments?.q ?? "");
    executions.set(q, (executions.get(q) ?? 0) + 1);
    if (q.startsWith("fail-")) {
      return new Response("upstream exploded", { status: 500 });
    }
    if (q.startsWith("limited-")) {
      return new Response(null, {
        headers: { "retry-after": "7" },
        status: 429,
      });
    }
    if (q.startsWith("slow-")) {
      await Bun.sleep(1500);
    }
    return sse([
      {
        id: "ask-1",
        jsonrpc: "2.0",
        method: "elicitation/create",
        params: { message: "Pick one", requestedSchema: { type: "object" } },
      },
      {
        id: message.id,
        jsonrpc: "2.0",
        result: { content: [{ text: `docs for ${q}`, type: "text" }] },
      },
    ]);
  },
  hostname: "127.0.0.1",
  port: upstreamPort,
});

// --- Scenarios -----------------------------------------------------------------

const discovery = async (
  name: string,
  group: Account[],
  perAccount: number
) => {
  const tasks = group.flatMap((a) =>
    Array.from(
      { length: perAccount },
      (_, i) => () =>
        i % 2
          ? rpc(a.key, "tools/list", {}, { "x-hive-discovery": "all" })
          : call(a.key, "search_tools", {
              query: ["documentation", "web search", "templates", "memory"][
                i % 4
              ],
            })
    )
  );
  const { seconds, value } = await timed(() => pool(64, tasks));
  const statuses = summarize(value, seconds);
  const errors = value.filter((r) => r.status !== 200);
  check(
    name,
    errors.length === 0,
    `every discovery request succeeded (${errors.length} failed${errors[0] ? `, e.g. ${errors[0].status} ${JSON.stringify(errors[0].body).slice(0, 120)}` : ""})`
  );
  check(
    name,
    value.every((r) => r.status !== 200 || r.body?.result !== undefined),
    "every success carries a JSON-RPC result"
  );
  return statuses;
};

/** A spike far beyond `http.concurrency + queue_depth`: excess load is shed cleanly. */
const burst = async (group: Account[]) => {
  const tasks = group.flatMap((a) =>
    Array.from({ length: 50 }, () => () => rpc(a.key, "tools/list"))
  );
  const { seconds, value } = await timed(() => pool(tasks.length, tasks));
  const statuses = summarize(value, seconds);
  const shed = value.filter((r) => r.status === 503);
  check(
    "burst",
    (statuses.get(200) ?? 0) >= tasks.length / 2,
    "most of the spike is served"
  );
  check(
    "burst",
    value.every((r) => r.status === 200 || r.status === 503),
    "the rest is shed with 503, never errors or resets"
  );
  check(
    "burst",
    shed.every((r) => r.retryAfter === "1"),
    "shed responses carry Retry-After"
  );
};

const rateLimit = async (group: Account[]) => {
  const a = only1(group);
  const tasks = Array.from(
    { length: 700 },
    () => () => rpc(a.key, "tools/list")
  );
  const { seconds, value } = await timed(() => pool(16, tasks));
  const statuses = summarize(value, seconds);
  const limited = value.filter((r) => r.status === 429);
  check(
    "rate limit",
    (statuses.get(200) ?? 0) <= 600,
    "no more than 600 requests per credential per minute"
  );
  check(
    "rate limit",
    limited.length >= 100,
    `excess requests are rejected (${limited.length} × 429)`
  );
  check(
    "rate limit",
    limited.every(
      (r) => Number(r.retryAfter) > 0 && Number(r.retryAfter) <= 60
    ),
    "429 responses carry Retry-After"
  );
  check(
    "rate limit",
    value.every((r) => r.status < 500),
    "the limiter never returns 5xx"
  );
};

interface Worker {
  stop: () => void;
  done: Promise<void>;
}
/** A connector worker that long-polls, echoes arguments and records every job it saw. */
const worker = (
  a: Account,
  connection: string,
  seen: Map<string, number>
): Worker => {
  const control = { running: true };
  const done = (async () => {
    while (control.running) {
      // oxlint-disable-next-line no-await-in-loop -- A worker long-polls one job at a time.
      const polled = await call(a.key, "poll_mcp", {
        connection_id: connection,
        wait_seconds: 2,
      });
      const job = polled.result?.structuredContent?.job;
      if (polled.status !== 200) {
        // oxlint-disable-next-line no-await-in-loop -- Back off before polling again.
        await Bun.sleep(200);
        continue;
      }
      if (!job) {
        continue;
      }
      seen.set(job.id, (seen.get(job.id) ?? 0) + 1);
      // oxlint-disable-next-line no-await-in-loop -- Completion follows its own claim.
      const done_ = await call(a.key, "complete_mcp", {
        connection_id: connection,
        job_id: job.id,
        lease: job.lease,
        result: {
          content: [
            {
              text: JSON.stringify(job.request?.arguments ?? null),
              type: "text",
            },
          ],
        },
      });
      if (done_.result?.isError) {
        console.log(`    worker completion rejected: ${textOf(done_.result)}`);
      }
    }
  })();
  return {
    done,
    stop: () => {
      control.running = false;
    },
  };
};
const register = async (a: Account, name: string) => {
  const registered = await call(a.key, "register_mcp_connection", {
    features: ["long_poll", "cancel"],
    name,
    tools: [{ inputSchema: { type: "object" }, name: "echo" }],
  });
  const id = registered.result?.structuredContent?.id;
  if (!id) {
    throw new Error(
      `register failed: ${JSON.stringify(registered.body).slice(0, 200)}`
    );
  }
  return id as string;
};
const connectors = async (
  name: string,
  group: Account[],
  workers: number,
  callers: number,
  rounds: number
) => {
  const seen = new Map<string, number>();
  const setups = await Promise.all(
    group.map(async (a) => {
      const connection = await register(a, "load");
      return {
        a,
        connection,
        workers: Array.from({ length: workers }, () =>
          worker(a, connection, seen)
        ),
      };
    })
  );
  let lost = 0;
  let mismatched = 0;
  let deferred = 0;
  const samples: Sample[] = [];
  const { seconds } = await timed(() =>
    Promise.all(
      setups.flatMap(({ a }) =>
        Array.from({ length: callers }, async () => {
          for (let round = 0; round < rounds; round += 1) {
            const id = crypto.randomUUID();
            // oxlint-disable-next-line no-await-in-loop -- Each caller issues its rounds sequentially.
            let reply = await call(a.key, "mcp_load__echo", { id });
            samples.push(reply);
            const task = reply.result?.structuredContent?.task?.taskId;
            if (task) {
              deferred += 1;
              // oxlint-disable-next-line no-await-in-loop -- Follows the task this round created.
              reply = await call(a.key, "get_task", {
                task_id: task,
                wait_seconds: 20,
              });
            }
            if (reply.status !== 200 || reply.result?.isError) {
              lost += 1;
              if (lost <= 3) {
                console.log(
                  `    lost call: ${reply.status} ${JSON.stringify(reply.body).slice(0, 200)}`
                );
              }
            } else if (!textOf(reply.result).includes(id)) {
              mismatched += 1;
            }
          }
        })
      )
    )
  );
  for (const setup of setups) {
    for (const w of setup.workers) {
      w.stop();
    }
  }
  await Promise.all(setups.flatMap((s) => s.workers.map((w) => w.done)));
  await Promise.all(
    setups.map(({ a, connection }) =>
      call(a.key, "disconnect_mcp", { connection_id: connection })
    )
  );
  summarize(samples, seconds);
  const total = group.length * callers * rounds;
  console.log(
    `  ${total} connector calls · ${(total / seconds).toFixed(0)} calls/s · ${deferred} continued as tasks`
  );
  check(name, lost === 0, `no connector call was lost (${lost})`);
  check(
    name,
    mismatched === 0,
    `every caller received its own result (${mismatched} mismatched)`
  );
  const duplicates = [...seen.values()].filter((n) => n > 1).length;
  check(
    name,
    duplicates === 0,
    `no job was delivered to two workers (${duplicates})`
  );
  check(
    name,
    seen.size === total,
    `workers saw exactly one job per call (${seen.size}/${total})`
  );
};

const queueFull = async (group: Account[]) => {
  const a = only1(group);
  const connection = await register(a, "stalled");
  const { seconds, value } = await timed(() =>
    Promise.all(
      Array.from({ length: 40 }, (_, i) =>
        call(a.key, "mcp_stalled__echo", { i })
      )
    )
  );
  summarize(value, seconds);
  const full = value.filter((r) =>
    JSON.stringify(r.body).includes("mcp_queue_full")
  );
  const tasks = value.filter((r) => r.result?.structuredContent?.task?.taskId);
  console.log(
    `  ${tasks.length} continued as tasks · ${full.length} rejected as queue full`
  );
  check(
    "queue full",
    tasks.length <= 16,
    "at most jobs_per_connection calls are queued"
  );
  check(
    "queue full",
    full.length >= 24,
    "calls beyond the bound are refused instead of piling up"
  );
  check(
    "queue full",
    value.every((r) => r.status === 200 || r.status === 503),
    "overload answers 200 (task) or 503, never hangs or 500"
  );
  await call(a.key, "disconnect_mcp", { connection_id: connection });
};

const hosted = async (group: Account[]) => {
  const saved = await Promise.all(
    group.map((a, i) =>
      request("/v1/account/providers/context7", a.session, {
        body: { api_key: `load-key-${i}` },
        method: "PUT",
      })
    )
  );
  check(
    "hosted provider",
    saved.every((r) => r.status === 200),
    `every account saved its key (${saved.map((r) => r.status).join(",")})`
  );
  const kinds = [
    "ok",
    "ok",
    "ok",
    "ok",
    "ok",
    "slow",
    "fail",
    "limited",
  ] as const;
  const plan = group.flatMap((a) =>
    Array.from({ length: 20 * scale }, (_, i) => ({
      a,
      kind: kinds[i % kinds.length] ?? "ok",
      q: `${kinds[i % kinds.length] ?? "ok"}-${crypto.randomUUID()}`,
    }))
  );
  const { seconds, value } = await timed(() =>
    pool(
      48,
      plan.map(
        (p) => () => call(p.a.key, "mcp_context7__query-docs", { q: p.q })
      )
    )
  );
  summarize(value, seconds);
  let wrong = 0;
  const outcomes = plan.map((p, i) => ({ p, r: value[i] }));
  // A full per-connection queue is a definite refusal: nothing was sent.
  const refused = new Set(
    outcomes
      .filter(({ r }) => textOf(r?.result).includes("mcp_queue_full"))
      .map(({ p }) => p.q)
  );
  console.log(`  ${refused.size} refused by the per-connection queue bound`);
  for (const { p, r } of outcomes) {
    const text = textOf(r?.result);
    if (refused.has(p.q) || !r) {
      wrong += r ? 0 : 1;
      continue;
    }
    const ok =
      r.status === 200 &&
      (p.kind === "ok" || p.kind === "slow"
        ? r.result?.isError !== true && text === `docs for ${p.q}`
        : r.result?.isError === true &&
          (p.kind === "fail"
            ? text.includes("outcome is unknown")
            : text.includes("rate limiting")));
    if (!ok) {
      wrong += 1;
      if (wrong <= 3) {
        console.log(
          `    unexpected ${p.kind}: ${r.status} ${JSON.stringify(r.body).slice(0, 220)}`
        );
      }
    }
  }
  check(
    "hosted provider",
    wrong === 0,
    `every call got the right outcome (${wrong} wrong)`
  );
  const repeated = plan.filter((p) => (executions.get(p.q) ?? 0) > 1).length;
  check(
    "hosted provider",
    repeated === 0,
    `no provider call executed twice despite session expiry and failures (${repeated})`
  );
  check(
    "hosted provider",
    refused.size <= plan.length / 4,
    "most calls fit the queue bound"
  );
  check(
    "hosted provider",
    [...refused].every((q) => !executions.has(q)),
    "refused calls never reached the provider"
  );
  const missing = plan.filter(
    (p) => !refused.has(p.q) && (executions.get(p.q) ?? 0) === 0
  ).length;
  check(
    "hosted provider",
    missing === 0,
    `every call reached the provider exactly once (${missing} never sent)`
  );
  console.log(
    `  provider saw ${providerRequests} HTTP requests for ${plan.length} calls`
  );
};

const memory = async (group: Account[]) => {
  const a = only1(group);
  const repo = `github.com/load/${runId}`;
  const writes = await Promise.all(
    Array.from({ length: 25 }, (_, i) =>
      call(a.key, "remember", {
        body: `Version ${i}`,
        kind: "decision",
        repo,
        title: "Deploy target",
      })
    )
  );
  check(
    "memory",
    writes.every((r) => r.status === 200 && r.result?.isError === false),
    `concurrent writes succeed (${writes
      .filter((r) => r.result?.isError !== false)
      .map((r) => textOf(r.result))
      .slice(0, 2)
      .join(" | ")})`
  );
  check(
    "memory",
    writes.filter((r) => r.result?.structuredContent?.created === true)
      .length === 1,
    "exactly one write creates the memory"
  );
  const recalled = await call(a.key, "recall", {
    include_global: false,
    repo,
  });
  const memories = recalled.result?.structuredContent?.memories ?? [];
  check(
    "memory",
    memories.length === 1,
    `one memory per title after a write race (${memories.length})`
  );
};

const oauth = async (group: Account[]) => {
  const a = only1(group);
  const client = await request("/oauth/register", "", {
    body: {
      application_type: "native",
      client_name: "Load test",
      redirect_uris: ["http://127.0.0.1/callback"],
    },
  });
  const verifier = "v".repeat(50);
  const challenge = new Bun.CryptoHasher("sha256")
    .update(verifier)
    .digest("base64url");
  const approved = await request("/v1/oauth/authorize", a.session, {
    body: {
      client_id: client.body.client_id,
      code_challenge: challenge,
      code_challenge_method: "S256",
      decision: "approve",
      redirect_uri: "http://127.0.0.1:8123/callback",
      resource: `${base}/mcp`,
      response_type: "code",
      scope: "capabilities:read memory:read",
      state: "load",
    },
  });
  const code = new URL(approved.body.redirect).searchParams.get("code");
  const tokens = await request("/oauth/token", "", {
    body: {
      client_id: client.body.client_id,
      code,
      code_verifier: verifier,
      grant_type: "authorization_code",
      redirect_uri: "http://127.0.0.1:8123/callback",
    },
  });
  check(
    "oauth",
    tokens.status === 200,
    `authorization code exchanged (${tokens.status})`
  );
  const refreshes = await Promise.all(
    Array.from({ length: 12 }, () =>
      request("/oauth/token", "", {
        body: {
          client_id: client.body.client_id,
          grant_type: "refresh_token",
          refresh_token: tokens.body.refresh_token,
        },
      })
    )
  );
  const winners = refreshes.filter((r) => r.status === 200);
  check(
    "oauth",
    winners.length <= 1,
    `a refresh token is redeemed at most once under a race (${winners.length})`
  );
  check(
    "oauth",
    refreshes.every((r) => r.status === 200 || r.status === 400),
    "losing redemptions are rejected, not 5xx"
  );
  if (winners.length === 1) {
    const [winner] = winners;
    const listed = await rpc(winner?.body.access_token, "tools/list");
    check(
      "oauth",
      listed.status === 401,
      "a reused refresh token revokes the whole grant"
    );
  }
};

const docker = async (...args: string[]) => {
  const child = Bun.spawn(["docker", ...args], {
    stderr: "pipe",
    stdout: "pipe",
  });
  return (await child.exited) === 0;
};
const redisOutage = async (group: Account[]) => {
  check("redis outage", await docker("pause", redisContainer), "Redis paused");
  try {
    const statuses = await discovery("redis outage", group, 20 * scale);
    check(
      "redis outage",
      !statuses.has(500) && !statuses.has(0),
      "requests are served without Redis"
    );
  } finally {
    await docker("unpause", redisContainer);
  }
  await Bun.sleep(1000);
  const after = await rpc(only1(group).key, "tools/list");
  check(
    "redis outage",
    after.status === 200,
    "service continues after Redis returns"
  );
};

const listenLoss = async (group: Account[]) => {
  const killed =
    await db`SELECT pg_terminate_backend(pid) AS done FROM pg_stat_activity WHERE datname = current_database() AND query ILIKE 'LISTEN%'`;
  check(
    "listen loss",
    killed.length > 0,
    `terminated ${killed.length} LISTEN connection(s)`
  );
  await connectors("listen loss", group, 2, 3, 3 * scale);
  await Bun.sleep(2500);
  const relisten =
    await db`SELECT count(*)::int AS n FROM pg_stat_activity WHERE datname = current_database() AND query ILIKE 'LISTEN%'`;
  check(
    "listen loss",
    (relisten[0]?.n ?? 0) > 0,
    "Hiveway re-established LISTEN"
  );
};

// --- Main ----------------------------------------------------------------------

const scenarios: [string, number, (group: Account[]) => Promise<unknown>][] = [
  ["discovery", 12, (g) => discovery("discovery", g, 40 * scale)],
  ["burst", 30, burst],
  ["rate-limit", 1, rateLimit],
  ["connectors", 10, (g) => connectors("connectors", g, 3, 6, 5 * scale)],
  ["queue-full", 1, queueFull],
  ["hosted", 8, hosted],
  ["memory", 1, memory],
  ["oauth", 1, oauth],
  ["redis-outage", 4, redisOutage],
  ["listen-loss", 3, listenLoss],
];
try {
  for (const [name, size, run] of scenarios) {
    if (only && !only.includes(name)) {
      continue;
    }
    console.log(`\n▸ ${name}`);
    try {
      // oxlint-disable-next-line no-await-in-loop -- Scenarios run one at a time so results stay attributable.
      await run(await accounts(size));
    } catch (error) {
      check(
        name,
        false,
        `scenario crashed: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }
} finally {
  provider.stop(true);
  await db.close();
}
console.log(
  failures.length
    ? `\n${failures.length} check(s) failed:\n- ${failures.join("\n- ")}`
    : "\nAll load checks passed."
);
process.exit(failures.length ? 1 : 0);
