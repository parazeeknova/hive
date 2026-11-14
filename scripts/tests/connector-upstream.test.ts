import { expect, test } from "bun:test";

import type { Transport } from "@modelcontextprotocol/client";
import { InMemoryTransport, Server } from "@modelcontextprotocol/server";

import type { HiveRpc } from "../../packages/connector/src/hive-rpc";
import { Upstream } from "../../packages/connector/src/upstream";

/** An upstream whose only tool asks the user to confirm before answering. */
const confirmingUpstream = async () => {
  const [clientSide, serverSide] = InMemoryTransport.createLinkedPair();
  const server = new Server(
    { name: "fixture", version: "1" },
    { capabilities: { tools: {} } }
  );
  server.setRequestHandler("tools/list", () => ({
    tools: [{ inputSchema: { type: "object" }, name: "confirm" }],
  }));
  server.setRequestHandler("tools/call", async (_request, ctx) => {
    const answer = await ctx.mcpReq.elicitInput({
      message: "Proceed?",
      mode: "form",
      requestedSchema: {
        properties: { ok: { type: "boolean" } },
        type: "object",
      },
    });
    return { content: [{ text: JSON.stringify(answer), type: "text" }] };
  });
  await server.connect(serverSide);
  const upstream = new Upstream({
    name: "fixture",
    transport: () => clientSide,
  });
  await upstream.connect();
  return { server, upstream };
};
/** A Hive stand-in recording connector operations. */
const hive = (answer: Record<string, unknown>) => {
  const calls: { name: string; args: Record<string, unknown> }[] = [];
  const rpc = {
    operation: (name: string, args: Record<string, unknown>) => {
      calls.push({ args, name });
      if (name === "elicit_mcp") {
        return Promise.resolve({ accepted: true, forwarded: true });
      }
      if (name === "await_input_mcp") {
        return Promise.resolve(answer);
      }
      return Promise.resolve({ cancel_requested: false });
    },
  } as unknown as HiveRpc;
  return { calls, rpc };
};
const job = {
  deadline_ms: Date.now() + 30_000,
  id: "job",
  lease: "lease",
  request: { arguments: {}, name: "confirm" },
};

test("upstream elicitation reaches the Hive caller and the answer returns upstream", async () => {
  const { server, upstream } = await confirmingUpstream();
  const { calls, rpc } = hive({
    responses: { input: { action: "accept", content: { ok: true } } },
  });
  try {
    const outcome = await upstream.run(rpc, "connection", job, 1);
    expect("result" in outcome).toBe(true);
    const text = JSON.stringify("result" in outcome ? outcome.result : {});
    expect(text).toContain("accept");
    const asked = calls.find((call) => call.name === "elicit_mcp");
    expect(asked?.args.connection_id).toBe("connection");
    const requests = asked?.args.requests as
      | { input: { params: { message: string } } }
      | undefined;
    expect(requests?.input.params.message).toBe("Proceed?");
  } finally {
    await upstream.close();
    await server.close();
  }
});

test("a caller cancellation during input aborts the upstream call", async () => {
  const { server, upstream } = await confirmingUpstream();
  const { rpc } = hive({ cancelled: true });
  try {
    const outcome = await upstream.run(rpc, "connection", job, 1);
    expect(outcome).toMatchObject({ cancelled: true });
  } finally {
    await upstream.close();
    await server.close();
  }
});

/** A deterministic transport fault without invoking or logging real provider data. */
const faultingUpstream = (readOnlyTools: string[], failures: number) => {
  let attempts = 0;
  let repairs = 0;
  const upstream = new Upstream({
    name: "fixture",
    readOnlyTools,
    transport: () => InMemoryTransport.createLinkedPair()[0],
  });
  upstream.client.callTool = () => {
    attempts += 1;
    if (attempts <= failures) {
      return Promise.reject(
        new Error("sensitive-provider-error-must-not-be-logged")
      );
    }
    return Promise.resolve({ content: [{ text: "recovered", type: "text" }] });
  };
  upstream.repair = () => {
    repairs += 1;
    return Promise.resolve();
  };
  return { attempts: () => attempts, repairs: () => repairs, upstream };
};

test("one operator-verified read retries after session repair", async () => {
  const fixture = faultingUpstream(["confirm"], 1);
  const { rpc } = hive({});
  const outcome = await fixture.upstream.run(
    rpc,
    "connection",
    { ...job, deadline_ms: Date.now() + 30_000 },
    1
  );
  expect(outcome).toMatchObject({
    result: { content: [{ text: "recovered", type: "text" }] },
  });
  expect(fixture.attempts()).toBe(2);
  expect(fixture.repairs()).toBe(1);
});

test("mutations and unverified tools are never repeated", async () => {
  const fixture = faultingUpstream([], 1);
  const { rpc } = hive({});
  const outcome = await fixture.upstream.run(
    rpc,
    "connection",
    { ...job, deadline_ms: Date.now() + 30_000 },
    1
  );
  expect(outcome).toMatchObject({ result: { isError: true } });
  expect(fixture.attempts()).toBe(1);
  expect(fixture.repairs()).toBe(1);
});

test("a read retry has a hard one-attempt ceiling", async () => {
  const fixture = faultingUpstream(["confirm"], 3);
  const { rpc } = hive({});
  const outcome = await fixture.upstream.run(
    rpc,
    "connection",
    { ...job, deadline_ms: Date.now() + 30_000 },
    1
  );
  expect(outcome).toMatchObject({ result: { isError: true } });
  expect(fixture.attempts()).toBe(2);
});

test("an expired read is not retried after repair", async () => {
  const fixture = faultingUpstream(["confirm"], 1);
  const { rpc } = hive({});
  await fixture.upstream.run(
    rpc,
    "connection",
    { ...job, deadline_ms: Date.now() - 1 },
    1
  );
  expect(fixture.attempts()).toBe(0);
});

test("application error envelopes are returned without replay", async () => {
  const fixture = faultingUpstream(["confirm"], 0);
  fixture.upstream.client.callTool = () =>
    Promise.resolve({ content: [], isError: true });
  const { rpc } = hive({});
  const outcome = await fixture.upstream.run(rpc, "connection", job, 1);
  expect(outcome).toMatchObject({ result: { isError: true } });
  expect(fixture.repairs()).toBe(0);
});

/** Real sessions exercise SDK catalog caching, validation and abort behavior. */
const replacementUpstream = async (
  replacement: "invalid" | "failure" | "silent"
) => {
  const calls = [0, 0, 0];
  const lists = [0, 0, 0];
  const servers: Server[] = [];
  const transports: Transport[] = [];
  const connections: Promise<void>[] = [];
  for (const index of [0, 1, 2]) {
    const [clientSide, serverSide] = InMemoryTransport.createLinkedPair();
    const server = new Server(
      { name: "replacement-fixture", version: "1" },
      { capabilities: { tools: {} } }
    );
    server.setRequestHandler("tools/list", () => {
      lists[index] = (lists[index] ?? 0) + 1;
      return {
        tools: ["confirm", "write"].map((name) => ({
          inputSchema: { type: "object" as const },
          name,
          outputSchema: {
            properties: { answer: { type: "integer" } },
            required: ["answer"],
            type: "object" as const,
          },
        })),
      };
    });
    server.setRequestHandler("tools/call", () => {
      calls[index] = (calls[index] ?? 0) + 1;
      if (index === 0 || (index === 1 && replacement === "failure")) {
        throw new Error("session fault");
      }
      if (index === 1 && replacement === "silent") {
        // oxlint-disable-next-line promise/avoid-new -- Deliberately silent upstream; the client must enforce its deadline.
        return new Promise<never>(() => {});
      }
      return {
        content: [],
        structuredContent: { answer: index === 1 ? "invalid" : 42 },
      };
    });
    connections.push(server.connect(serverSide));
    servers.push(server);
    transports.push(clientSide);
  }
  await Promise.all(connections);
  let index = 0;
  const upstream = new Upstream({
    name: "fixture",
    readOnlyTools: ["confirm"],
    transport: () => {
      const next = transports[index];
      index += 1;
      if (!next) {
        throw new Error("Unexpected extra session");
      }
      return next;
    },
  });
  await upstream.connect();
  await upstream.snapshot();
  return {
    calls,
    close: async () => {
      await upstream.close();
      await Promise.all(servers.map((server) => server.close()));
    },
    lists,
    upstream,
  };
};

test("replacement clients validate retry output against the refreshed catalog", async () => {
  const fixture = await replacementUpstream("invalid");
  try {
    const outcome = await fixture.upstream.run(
      hive({}).rpc,
      "connection",
      { ...job, deadline_ms: Date.now() + 30_000 },
      1
    );
    expect(outcome).toMatchObject({ result: { isError: true } });
    expect(fixture.calls).toEqual([1, 1, 0]);
    expect(fixture.lists).toEqual([1, 1, 0]);
  } finally {
    await fixture.close();
  }
});

test("a failed retry repairs before the next unverified tool is dispatched", async () => {
  const fixture = await replacementUpstream("failure");
  try {
    const outcome = await fixture.upstream.run(
      hive({}).rpc,
      "connection",
      { ...job, deadline_ms: Date.now() + 30_000 },
      1
    );
    expect(outcome).toMatchObject({ result: { isError: true } });
    expect(fixture.calls).toEqual([1, 1, 0]);
    const next = await fixture.upstream.run(
      hive({}).rpc,
      "connection",
      {
        ...job,
        deadline_ms: Date.now() + 30_000,
        request: { arguments: {}, name: "write" },
      },
      1
    );
    expect(next).toMatchObject({
      result: { structuredContent: { answer: 42 } },
    });
    expect(fixture.calls).toEqual([1, 1, 1]);
  } finally {
    await fixture.close();
  }
});

test("a silent retry is aborted at the absolute job deadline", async () => {
  const fixture = await replacementUpstream("silent");
  try {
    const started = Date.now();
    const outcome = await fixture.upstream.run(
      hive({}).rpc,
      "connection",
      { ...job, deadline_ms: started + 2400 },
      1
    );
    expect(outcome).toMatchObject({ cancelled: true });
    expect(fixture.calls).toEqual([1, 1, 0]);
    expect(Date.now() - started).toBeLessThan(4000);
  } finally {
    await fixture.close();
  }
}, 5000);
