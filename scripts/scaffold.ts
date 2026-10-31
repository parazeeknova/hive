/* oxlint-disable no-await-in-loop -- Filesystem writes, queue polling and API-first deployment must run in order. */
/** Safely apply any pinned public GitHub starter; never overwrite an existing target. */
import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";
import { gunzipSync } from "node:zlib";

import { Parser } from "tar";
import type { ReadEntry } from "tar";

const MAX_ARCHIVE = 8 * 1024 * 1024;
const MAX_CONTENT = 32 * 1024 * 1024;
const MAX_FILES = 2000;
export interface Plan {
  archive_sha256: string;
  archive_url: string;
  project_name: string;
  revision: string;
}
interface Entry {
  content: Buffer;
  executable: boolean;
}
export const checksum = (bytes: Uint8Array) =>
  createHash("sha256").update(bytes).digest("hex");
export const verifyBundle = async (
  plan: Pick<Plan, "archive_sha256">,
  archive: Uint8Array
) => {
  if (
    archive.length > MAX_ARCHIVE ||
    checksum(archive) !== plan.archive_sha256
  ) {
    throw new Error(
      "Template archive exceeds download limit or checksum mismatch"
    );
  }
  const expanded = gunzipSync(archive, { maxOutputLength: MAX_CONTENT });
  const files = new Map<string, Entry>();
  let root = "";
  let total = 0;
  let entries = 0;
  const { promise, resolve, reject } = Promise.withResolvers<undefined>();
  {
    const parser = new Parser({ maxMetaEntrySize: 16_384, strict: true });
    parser.on("error", reject);
    parser.on("end", () => resolve(undefined));
    parser.on("entry", (entry: ReadEntry) => {
      try {
        entries += 1;
        const parts = entry.path.split("/").filter(Boolean);
        if (
          entries > MAX_FILES * 2 ||
          entry.path.startsWith("/") ||
          entry.path.includes("\\") ||
          parts.some(
            (part) =>
              part === ".." ||
              part.includes(":") ||
              part.replace(/[ .]+$/u, "").toLowerCase() === ".git"
          ) ||
          !parts.length
        ) {
          throw new Error("Unsafe template path or too many entries");
        }
        const source = parts.shift();
        if (!source) {
          throw new Error("Missing source root");
        }
        root ||= source;
        if (root !== source || !["File", "Directory"].includes(entry.type)) {
          throw new Error(
            "Links, special files and multiple source roots are not supported"
          );
        }
        if (entry.type === "Directory") {
          entry.resume();
          return;
        }
        const name = parts.join("/");
        total += entry.size;
        if (
          !name ||
          files.has(name) ||
          files.size >= MAX_FILES ||
          total > MAX_CONTENT ||
          entry.size < 0
        ) {
          throw new Error(
            "Invalid path, duplicate file or template limit exceeded"
          );
        }
        const chunks: Buffer[] = [];
        // Reserve names before the stream ends so duplicates cannot replace entries.
        const file: Entry = {
          content: Buffer.alloc(0),
          // oxlint-disable-next-line no-bitwise -- Unix executable permission bits.
          executable: Boolean((entry.mode ?? 0) & 0o111),
        };
        files.set(name, file);
        entry.on("data", (chunk: Buffer) => {
          chunks.push(chunk);
        });
        entry.on("end", () => {
          file.content = Buffer.concat(chunks);
          if (file.content.length !== entry.size) {
            parser.abort(new Error("Truncated template file"));
          }
        });
        entry.resume();
      } catch (error) {
        parser.abort(
          error instanceof Error ? error : new Error("Invalid archive")
        );
      }
    });
    parser.end(expanded);
  }
  await promise;
  if (!files.size) {
    throw new Error("Template contains no files");
  }
  return files;
};
export const applyBundle = async (
  plan: Pick<Plan, "archive_sha256">,
  archive: Uint8Array,
  target: string,
  operations: Pick<typeof fs, "writeFile" | "chmod"> = fs
) => {
  const files = await verifyBundle(plan, archive);
  await fs.mkdir(target, { mode: 0o755 });
  try {
    for (const [name, file] of files) {
      const destination = path.join(target, name);
      await fs.mkdir(path.dirname(destination), { recursive: true });
      await operations.writeFile(destination, file.content, { flag: "wx" });
      await operations.chmod(destination, file.executable ? 0o755 : 0o644);
    }
  } catch (error) {
    await fs.rm(target, { force: true, recursive: true }).catch(() => {
      /* Preserve the original write error if cleanup also fails. */
    });
    throw error;
  }
  return files.size;
};
const fetchBounded = async (url: string, key?: string, body?: unknown) => {
  const response = await fetch(url, {
    ...(body === undefined
      ? {}
      : { body: JSON.stringify(body), method: "POST" }),
    headers: {
      Accept: "application/json, application/gzip",
      "Content-Type": "application/json",
      "User-Agent": "Hive-scaffold/1.0",
      ...(key ? { Authorization: `Bearer ${key}` } : {}),
    },
    redirect: "error",
    signal: AbortSignal.timeout(30_000),
  });
  if (!response.ok || !response.body) {
    throw new Error(`Scaffold request failed (${response.status})`);
  }
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      size += value.length;
      if (size > MAX_ARCHIVE) {
        throw new Error("Scaffold response exceeds size limit");
      }
      chunks.push(value);
    }
  } finally {
    await reader.cancel();
  }
  return Buffer.concat(chunks);
};
const argumentsForScaffold = () => {
  const [repository, revision, target] = process.argv.slice(2);
  const source = repository?.match(
    /^https:\/\/github\.com\/(?<owner>[\w.-]+)\/(?<repo>[\w.-]+)\/?$/u
  );
  if (
    !source ||
    !source.groups?.owner ||
    !source.groups.repo ||
    [source.groups.owner, source.groups.repo].some(
      (part) => part === "." || part === ".."
    ) ||
    !revision ||
    !/^[a-f\d]{40}$/iu.test(revision) ||
    !target
  ) {
    throw new Error(
      "Usage: bun scripts/scaffold.ts PUBLIC_GITHUB_URL FULL_COMMIT_SHA NEW_DIRECTORY"
    );
  }
  return {
    owner: source.groups.owner,
    repo: source.groups.repo,
    repository,
    revision,
    target,
  };
};
const scaffoldOrigin = () => {
  const origin = new URL(
    process.env.HIVE_ORIGIN ?? "https://hiveway.itssingularity.com"
  );
  if (
    origin.protocol !== "https:" ||
    origin.pathname !== "/" ||
    origin.search ||
    origin.hash ||
    origin.username ||
    origin.password
  ) {
    throw new Error("HIVE_ORIGIN must be an HTTPS origin");
  }
  return origin;
};
const main = async () => {
  const { repository, revision, target, owner, repo } = argumentsForScaffold();
  const name = path.basename(path.resolve(target));
  if (!/^[a-z\d][a-z\d_-]{0,62}$/iu.test(name)) {
    throw new Error(
      "Use letters, digits, hyphens or underscores in the directory name"
    );
  }
  const exists = await fs.lstat(target).then(
    () => true,
    (error: NodeJS.ErrnoException) => {
      if (error.code === "ENOENT") {
        return false;
      }
      throw error;
    }
  );
  if (exists) {
    throw new Error("Target already exists");
  }
  const key = process.env.HIVE_API_KEY?.trim();
  if (!key) {
    throw new Error("Set HIVE_API_KEY");
  }
  const origin = scaffoldOrigin();
  const responseBytes = await fetchBounded(`${origin.origin}/mcp`, key, {
    id: 1,
    jsonrpc: "2.0",
    method: "tools/call",
    params: {
      arguments: {
        project_name: name,
        repository_url: repository,
        revision,
      },
      name: "scaffold_project",
    },
  });
  const response = JSON.parse(responseBytes.toString()) as {
    error?: unknown;
    result?: { isError?: boolean; structuredContent?: Plan };
  };
  const plan = response.result?.structuredContent;
  const url = `https://codeload.github.com/${owner}/${repo}/tar.gz/${revision}`;
  if (
    response.error ||
    response.result?.isError ||
    !plan ||
    plan.archive_url !== url ||
    plan.project_name !== name ||
    plan.revision !== revision
  ) {
    throw new Error(
      "Hive rejected the request or returned an unexpected template"
    );
  }
  const count = await applyBundle(plan, await fetchBounded(url), target);
  const git = Bun.spawn(["git", "init", target], {
    stderr: "inherit",
    stdout: "inherit",
  });
  if (await git.exited) {
    throw new Error("Files created, but git init failed");
  }
  process.stdout.write(
    `Created ${target}: ${count} files from ${revision}\nRead the source README for prerequisites and setup. No dependency installation or template scripts were executed.\n`
  );
};
if (import.meta.main) {
  await main();
}
