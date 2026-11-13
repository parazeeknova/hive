import { expect, test } from "bun:test";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

test.each(["stdio", "filesystem"] as const)(
  "%s connector routes filesystem calls and enforces directory access",
  async (mode) => {
    const temp = await mkdtemp(path.join(os.tmpdir(), "hive-connector-"));
    const workspace = path.join(temp, "workspace");
    await mkdir(workspace);
    const keyFile = path.join(temp, "key");
    await writeFile(keyFile, "fixture-private-key");
    const jobs =
      mode === "filesystem"
        ? [
            {
              arguments: { directory: workspace },
              name: "request_directory_access",
            },
            { arguments: { path: keyFile }, name: "read_text_file" },
            {
              arguments: { path: path.join(workspace, "example.txt") },
              name: "read_text_file",
            },
          ]
        : [
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
    let failedPoll = false;
    let failedCompletion = false;
    const waits: unknown[] = [];
    const server = Bun.serve({
      async fetch(request) {
        // Public limits are read without credentials, before registration.
        if (new URL(request.url).pathname === "/api/config") {
          return Response.json({
            connectors: { poll_wait_seconds: 1, result_bytes: 524_288 },
          });
        }
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
          case "progress_mcp": {
            data = { cancel_requested: false };
            break;
          }
          case "poll_mcp": {
            waits.push(body.params.arguments.wait_seconds);
            if (!failedPoll) {
              failedPoll = true;
              return new Response("unavailable", { status: 503 });
            }
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
            results.push(
              body.params.arguments.result as Record<string, unknown>
            );
            if (!failedCompletion) {
              failedCompletion = true;
              return Response.json({
                id: body.id,
                jsonrpc: "2.0",
                result: {
                  isError: true,
                  structuredContent: { error: "stale_mcp_job" },
                },
              });
            }
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
        transport: mode,
        workingDirectory: workspace,
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
      expect(failedPoll && failedCompletion).toBe(true);
      expect(registered).toBe(true);
      expect(authorized).toBe(true);
      // The worker long-polls with the wait the deployment publishes.
      expect(waits.every((wait) => wait === 1)).toBe(true);
      expect(results).toHaveLength(3);
      if (mode === "stdio") {
        expect(
          await readFile(path.join(workspace, "example.txt"), "utf-8")
        ).toBe("hello hive");
        expect(JSON.stringify(results[1])).toContain("hello hive");
      } else {
        expect(results[0]?.isError).toBe(true);
        expect(results[1]?.isError).toBe(true);
      }
      expect(results[2]?.isError).toBe(true);
      expect(JSON.stringify(results[2])).not.toContain("fixture-private-key");
    } finally {
      child.kill();
      await child.exited;
      server.stop(true);
      await rm(temp, { force: true, recursive: true });
    }
  },
  20_000
);
