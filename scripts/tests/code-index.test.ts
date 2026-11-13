import { expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import {
  groupBlobs,
  parseBatch,
  syncRepository,
} from "../../packages/connector/src/code-index";
import { HiveRpc } from "../../packages/connector/src/hive-rpc";

const git = async (cwd: string, ...args: string[]) => {
  const child = Bun.spawn(["git", ...args], {
    cwd,
    env: {
      ...process.env,
      GIT_AUTHOR_EMAIL: "t@example.com",
      GIT_AUTHOR_NAME: "t",
      GIT_COMMITTER_EMAIL: "t@example.com",
      GIT_COMMITTER_NAME: "t",
    },
    stderr: "ignore",
    stdout: "ignore",
  });
  expect(await child.exited).toBe(0);
};
const blobId = (content: string) =>
  createHash("sha1")
    .update(`blob ${Buffer.byteLength(content)}\0`)
    .update(content)
    .digest("hex");

test("code index sync sends a chunked manifest and only missing text blobs", async () => {
  const repo = await mkdtemp(path.join(os.tmpdir(), "hive-code-index-"));
  const source = "export const answer = () => 42;\n";
  await mkdir(path.join(repo, "src"));
  await mkdir(path.join(repo, "node_modules", "dep"), { recursive: true });
  await writeFile(path.join(repo, "src", "app.ts"), source);
  await writeFile(path.join(repo, "README.md"), "# Demo\n");
  await writeFile(
    path.join(repo, "logo.png"),
    new Uint8Array([137, 80, 78, 71, 0, 1])
  );
  await writeFile(path.join(repo, "node_modules", "dep", "index.js"), "x\n");
  await git(repo, "init", "-q", "-b", "main");
  await git(repo, "add", "-A", "-f");
  await git(repo, "commit", "-q", "-m", "init");
  const manifests: Record<string, unknown>[] = [];
  const uploads: { sha: string; content: string; path: string }[] = [];
  let finalized = false;
  const server = Bun.serve({
    async fetch(request) {
      if (new URL(request.url).pathname === "/api/config") {
        return Response.json({
          connectors: {
            code_blobs_per_upload: 1,
            code_file_bytes: 10_000,
            code_manifest_files_per_call: 1,
          },
        });
      }
      const body = (await request.json()) as {
        id: string;
        params: { name: string; arguments: Record<string, unknown> };
      };
      const args = body.params.arguments;
      let data: Record<string, unknown> = {};
      if (body.params.name === "index_repository" && args.finalize) {
        finalized = true;
        data = { repo: "local/demo", status: "ready", summary: { stack: [] } };
      } else if (body.params.name === "index_repository") {
        manifests.push(args);
        const files = args.files as { sha: string }[];
        // README is already indexed from an earlier commit.
        data = {
          missing: files
            .map((file) => file.sha)
            .filter((sha) => sha !== blobId("# Demo\n")),
        };
      } else if (body.params.name === "upload_code_blobs") {
        for (const blob of args.blobs as typeof uploads) {
          expect(blobId(blob.content)).toBe(blob.sha);
          uploads.push(blob);
        }
        data = { stored: (args.blobs as unknown[]).length };
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
  try {
    const result = await syncRepository(
      new HiveRpc(`http://127.0.0.1:${server.port}/mcp`, "key"),
      repo
    );
    // Binary files and vendored directories never leave the machine.
    expect(result.files).toBe(2);
    expect(manifests).toHaveLength(2);
    expect(manifests.map((m) => m.reset)).toEqual([true, false]);
    expect(manifests[0]?.ref).toBe("main");
    expect(uploads.map((u) => u.path)).toEqual(["src/app.ts"]);
    expect(uploads[0]?.content).toBe(source);
    expect(result.uploaded).toBe(1);
    expect(finalized).toBe(true);
    expect(result.status).toBe("ready");
  } finally {
    server.stop(true);
    await rm(repo, { force: true, recursive: true });
  }
});

test("batch output parsing drops binary blobs and request groups stay bounded", () => {
  const text = "hello\n";
  const output = Buffer.concat([
    Buffer.from(`${blobId(text)} blob ${text.length}\n${text}\n`),
    Buffer.from("deadbeef blob 3\n"),
    Buffer.from([0, 1, 2]),
    Buffer.from("\n"),
  ]);
  const blobs = parseBatch(output);
  expect(blobs.get(blobId(text))).toBe(text);
  expect(blobs.get("deadbeef")).toBeNull();
  const blob = { content: "x".repeat(40), path: "a", sha: "s" };
  const groups = groupBlobs([blob, blob, blob], 120);
  expect(groups.map((g) => g.length)).toEqual([1, 1, 1]);
  expect(groupBlobs([blob, blob], 10_000)).toHaveLength(1);
});
