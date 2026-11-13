/**
 * Sync a git repository's committed HEAD into the Hive code index.
 *
 * The manifest comes from `git ls-tree`, so blob ids are git's own and nothing
 * is read for files Hive already has (from any earlier commit, machine or CI
 * run of the same account). Only missing blobs are read with one
 * `git cat-file --batch` per upload batch and sent in bounded requests.
 *
 * CLI: `bun packages/connector/src/code-index.ts --dir . --hive https://host/mcp --key-file ~/.config/hive/hive.key`
 * or set `HIVE_URL` and `HIVE_API_KEY` (for CI secrets).
 */
import { readFile } from "node:fs/promises";
import path from "node:path";

import { HiveRpc } from "./hive-rpc";

export interface SyncResult {
  repo: string;
  commit: string;
  ref: string | null;
  files: number;
  uploaded: number;
  skipped: number;
  status: string;
  summary: unknown;
}
interface Entry {
  path: string;
  sha: string;
  bytes: number;
}

const SKIPPED_DIRECTORIES = new Set([
  "node_modules",
  "vendor",
  "dist",
  "build",
  "target",
  ".next",
  "coverage",
  ".venv",
  "__pycache__",
]);
const BINARY =
  /\.(?:png|jpe?g|gif|webp|avif|ico|bmp|tiff?|pdf|zip|gz|tgz|bz2|xz|7z|rar|jar|war|wasm|woff2?|ttf|otf|eot|mp[34]|m4a|wav|flac|ogg|mov|avi|mkv|webm|so|dylib|dll|exe|bin|o|a|class|pyc|lockb|psd|sketch|fig|db|sqlite3?|parquet|onnx|pt|ckpt|safetensors)$/iu;

const git = async (directory: string, args: string[]) => {
  const child = Bun.spawn(["git", "-C", directory, ...args], {
    stderr: "pipe",
    stdout: "pipe",
  });
  const [output, code] = await Promise.all([
    new Response(child.stdout).arrayBuffer(),
    child.exited,
  ]);
  if (code !== 0) {
    throw new Error(`git ${args[0]} failed in ${directory}`);
  }
  return Buffer.from(output);
};
/** Files worth indexing: committed text blobs within the size limit. */
export const parseTree = (output: string, maxBytes: number): Entry[] =>
  output
    .split("\0")
    .filter(Boolean)
    .flatMap((record) => {
      const tab = record.indexOf("\t");
      const [mode, type, sha, size] = record.slice(0, tab).split(/\s+/u);
      const file = record.slice(tab + 1);
      const bytes = Number(size);
      const skipped =
        type !== "blob" ||
        mode === "120000" ||
        !sha ||
        !/^[0-9a-f]{40}$/u.test(sha) ||
        !Number.isFinite(bytes) ||
        bytes > maxBytes ||
        BINARY.test(file) ||
        file.split("/").some((part) => SKIPPED_DIRECTORIES.has(part));
      return skipped ? [] : [{ bytes, path: file, sha }];
    });
/** Parse `git cat-file --batch` output into UTF-8 text, dropping binary content. */
export const parseBatch = (output: Buffer) => {
  const blobs = new Map<string, string | null>();
  const decoder = new TextDecoder("utf-8", { fatal: true });
  let offset = 0;
  while (offset < output.length) {
    const end = output.indexOf(10, offset);
    if (end === -1) {
      break;
    }
    const [sha, type, size] = output
      .subarray(offset, end)
      .toString("utf-8")
      .split(" ");
    offset = end + 1;
    if (!sha || type !== "blob") {
      continue;
    }
    const length = Number(size);
    const content = output.subarray(offset, offset + length);
    offset += length + 1;
    try {
      const text = decoder.decode(content);
      blobs.set(sha, text.includes("\0") ? null : text);
    } catch {
      blobs.set(sha, null);
    }
  }
  return blobs;
};
const readBlobs = async (directory: string, shas: string[]) => {
  const child = Bun.spawn(["git", "-C", directory, "cat-file", "--batch"], {
    stderr: "pipe",
    stdin: "pipe",
    stdout: "pipe",
  });
  child.stdin.write(`${shas.join("\n")}\n`);
  await child.stdin.end();
  const [output, code] = await Promise.all([
    new Response(child.stdout).arrayBuffer(),
    child.exited,
  ]);
  if (code !== 0) {
    throw new Error("git cat-file failed");
  }
  return parseBatch(Buffer.from(output));
};
const gitText = async (directory: string, args: string[]) => {
  const output = await git(directory, args);
  return output.toString("utf-8").trim();
};
interface SourceBlob {
  sha: string;
  path: string;
  content: string;
}
/** Split blobs into requests that stay under the request body budget. */
export const groupBlobs = (blobs: SourceBlob[], budget: number) => {
  const groups: SourceBlob[][] = [];
  let current: SourceBlob[] = [];
  let bytes = 0;
  for (const blob of blobs) {
    const size = JSON.stringify(blob).length;
    if (current.length > 0 && bytes + size > budget) {
      groups.push(current);
      current = [];
      bytes = 0;
    }
    current.push(blob);
    bytes += size;
  }
  if (current.length > 0) {
    groups.push(current);
  }
  return groups;
};
const repositoryName = async (directory: string) => {
  try {
    const remote = await gitText(directory, ["remote", "get-url", "origin"]);
    if (remote) {
      return remote;
    }
  } catch {
    // No remote: fall back to a local name.
  }
  const base = path
    .basename(directory)
    .toLowerCase()
    .replaceAll(/[^a-z0-9._-]/gu, "-");
  return `local/${base || "repository"}`;
};

interface Target {
  repo: string;
  commit: string;
  ref: string | null;
}
const refField = (target: Target) => (target.ref ? { ref: target.ref } : {});
/** Send the manifest in ordered chunks and collect the blob ids Hive lacks. */
const sendManifest = async (
  rpc: HiveRpc,
  target: Target,
  entries: Entry[],
  chunk: number
) => {
  const missing = new Set<string>();
  const starts = entries.length === 0 ? [0] : [];
  for (let start = 0; start < entries.length; start += chunk) {
    starts.push(start);
  }
  for (const start of starts) {
    // oxlint-disable-next-line no-await-in-loop -- Manifest chunks are ordered: the first one resets the snapshot.
    const indexed = await rpc.operation(
      "index_repository",
      {
        commit: target.commit,
        files: entries.slice(start, start + chunk),
        repo: target.repo,
        reset: start === 0,
        ...refField(target),
      },
      60_000
    );
    for (const sha of (indexed.missing as string[] | undefined) ?? []) {
      missing.add(sha);
    }
  }
  return [...missing];
};
/** Upload missing blobs in bounded batches. */
const uploadMissing = async (
  rpc: HiveRpc,
  directory: string,
  entries: Entry[],
  pending: string[],
  limits: { perBatch: number; budget: number }
) => {
  const paths = new Map(entries.map((entry) => [entry.sha, entry.path]));
  const { budget, perBatch } = limits;
  let uploaded = 0;
  let skipped = 0;
  for (let start = 0; start < pending.length; start += perBatch) {
    const shas = pending.slice(start, start + perBatch);
    // oxlint-disable-next-line no-await-in-loop -- Bounded batches keep memory and request size predictable.
    const contents = await readBlobs(directory, shas);
    const blobs: SourceBlob[] = [];
    for (const sha of shas) {
      const content = contents.get(sha);
      const blob = { content: content ?? "", path: paths.get(sha) ?? "", sha };
      // Binary content and files that cannot fit one request are not indexed.
      if (typeof content !== "string" || JSON.stringify(blob).length > budget) {
        skipped += 1;
      } else {
        blobs.push(blob);
      }
    }
    for (const group of groupBlobs(blobs, budget)) {
      // oxlint-disable-next-line no-await-in-loop -- Requests stay sequential to respect the body limit and server load.
      const stored = await rpc.operation(
        "upload_code_blobs",
        { blobs: group },
        120_000
      );
      uploaded += Number(stored.stored ?? 0);
    }
  }
  return { skipped, uploaded };
};

/** Upload a repository's HEAD to the caller's Hive code index. */
export const syncRepository = async (
  rpc: HiveRpc,
  directory: string
): Promise<SyncResult> => {
  const { connectors, limits } = await rpc.config();
  const commit = await gitText(directory, ["rev-parse", "HEAD"]);
  const branch = await gitText(directory, [
    "rev-parse",
    "--abbrev-ref",
    "HEAD",
  ]);
  const target: Target = {
    commit,
    ref: branch && branch !== "HEAD" ? branch : null,
    repo: await repositoryName(directory),
  };
  const tree = await git(directory, ["ls-tree", "-r", "-l", "-z", "HEAD"]);
  const entries = parseTree(tree.toString("utf-8"), connectors.code_file_bytes);
  const pending = await sendManifest(
    rpc,
    target,
    entries,
    Math.max(1, connectors.code_manifest_files_per_call)
  );
  const { skipped, uploaded } = await uploadMissing(
    rpc,
    directory,
    entries,
    pending,
    {
      budget: Math.floor(limits.request_body_bytes * 0.7),
      perBatch: Math.max(1, connectors.code_blobs_per_upload),
    }
  );
  const finalized = await rpc.operation(
    "index_repository",
    { commit, finalize: true, repo: target.repo, ...refField(target) },
    60_000
  );
  return {
    commit,
    files: entries.length,
    ref: target.ref,
    repo: String(finalized.repo ?? target.repo),
    skipped,
    status: String(finalized.status ?? "pending"),
    summary: finalized.summary ?? null,
    uploaded,
  };
};

const flag = (name: string) => {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
};
const main = async () => {
  const hive = flag("--hive") ?? process.env.HIVE_URL;
  const keyFile = flag("--key-file");
  const key = keyFile
    ? await readFile(keyFile, "utf-8")
    : process.env.HIVE_API_KEY;
  if (!hive || !key) {
    throw new Error(
      "Usage: code-index.ts --dir <repo> --hive <https://host/mcp> --key-file <file> (or HIVE_URL and HIVE_API_KEY)"
    );
  }
  const directory = path.resolve(flag("--dir") ?? process.cwd());
  const result = await syncRepository(new HiveRpc(hive, key), directory);
  console.log(JSON.stringify(result, null, 2));
};
if (import.meta.main) {
  try {
    await main();
  } catch (error) {
    console.error(
      error instanceof Error ? error.message : "Code index sync failed"
    );
    process.exitCode = 1;
  }
}
