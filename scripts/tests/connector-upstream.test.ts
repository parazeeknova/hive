import { expect, test } from "bun:test";

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
