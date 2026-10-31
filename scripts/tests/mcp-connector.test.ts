import { expect, test } from "bun:test";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

test("connector routes real filesystem calls and rejects paths outside the workspace", async () => {
  const temp = await mkdtemp(path.join(os.tmpdir(), "hive-connector-"));
  const workspace = path.join(temp, "workspace");
  await mkdir(workspace);
  const keyFile = path.join(temp, "key");
  await writeFile(keyFile, "fixture-private-key");
  const jobs = [
    {
      arguments: {
        content: "hello hive",
        path: path.join(workspace, "example.txt"),
      },
      name: "write_file",
    },
    {
      arguments: { path: path.join(workspace, "example.txt") },
      name: "read_text_file",
    },
    { arguments: { path: keyFile }, name: "read_text_file" },
  ];
  const results: Record<string, unknown>[] = [];
  let authorized = true;
  let registered = false;
  const server = Bun.serve({
    async fetch(request) {
      authorized &&=
        request.headers.get("authorization") === "Bearer fixture-private-key";
      const body = (await request.json()) as {
        id: string;
        params: { name: string; arguments: Record<string, unknown> };
      };
      let data: Record<string, unknown> = {};
      switch (body.params.name) {
        case "register_mcp_connection": {
          registered = true;
          data = { id: crypto.randomUUID() };
          break;
        }
        case "poll_mcp": {
          data = {
            job: jobs.length
              ? {
                  id: crypto.randomUUID(),
                  lease: crypto.randomUUID(),
                  request: jobs.shift(),
                }
              : null,
          };
          break;
        }
        case "complete_mcp": {
          results.push(body.params.arguments.result as Record<string, unknown>);
          data = { completed: true };
          break;
        }
        default: {
          throw new Error("Unexpected gateway operation");
        }
      }
      return Response.json({
        id: body.id,
        jsonrpc: "2.0",
        result: { isError: false, structuredContent: data },
      });
    },
    hostname: "127.0.0.1",
    port: 0,
  });
  const config = path.join(temp, "config.json");
  await writeFile(
    config,
    JSON.stringify({
      args: [
        path.resolve(
          "node_modules/@modelcontextprotocol/server-filesystem/dist/index.js"
        ),
        workspace,
      ],
      command: process.execPath,
      hive: `http://127.0.0.1:${server.port}/mcp`,
      keyFile,
      name: "filesystem",
      transport: "stdio",
    })
  );
  const child = Bun.spawn(
    [process.execPath, "packages/connector/src/index.ts", config],
    { stderr: "ignore", stdout: "ignore" }
  );
  try {
    const deadline = Date.now() + 15_000;
    // These requests must execute in order to verify the write before the read.
    // eslint-disable-next-line no-await-in-loop
    while (results.length < 3 && Date.now() < deadline) {
      // eslint-disable-next-line no-await-in-loop -- Wait for the ordered worker results.
      await Bun.sleep(50);
    }
    expect(registered).toBe(true);
    expect(authorized).toBe(true);
    expect(results).toHaveLength(3);
    expect(await readFile(path.join(workspace, "example.txt"), "utf-8")).toBe(
      "hello hive"
    );
    expect(JSON.stringify(results[1])).toContain("hello hive");
    expect(results[2]?.isError).toBe(true);
    expect(JSON.stringify(results[2])).not.toContain("fixture-private-key");
  } finally {
    child.kill();
    await child.exited;
    server.stop(true);
    await rm(temp, { force: true, recursive: true });
  }
}, 20_000);
