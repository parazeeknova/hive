/* oxlint-disable no-await-in-loop -- Filesystem writes, queue polling and API-first deployment must run in order. */
import { describe, expect, test } from "bun:test";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { gzipSync } from "node:zlib";

import { create, Header } from "tar";

import { applyBundle, checksum, verifyBundle } from "../scaffold";

const bundle = async (entries: Record<string, string>) => {
  const parent = await fs.mkdtemp(path.join(os.tmpdir(), "hive-bundle-"));
  try {
    for (const [name, value] of Object.entries(entries)) {
      await fs.mkdir(path.dirname(path.join(parent, "root", name)), {
        recursive: true,
      });
      await fs.writeFile(path.join(parent, "root", name), value);
    }
    const archive = Buffer.concat(
      await Array.fromAsync(create({ cwd: parent, gzip: true }, ["root"]))
    );
    return { archive, plan: { archive_sha256: checksum(archive) } };
  } finally {
    await fs.rm(parent, { force: true, recursive: true });
  }
};
const temporary = async (fn: (target: string) => Promise<void>) => {
  const parent = await fs.mkdtemp(path.join(os.tmpdir(), "hive-scaffold-"));
  try {
    await fn(path.join(parent, "new-app"));
  } finally {
    await fs.rm(parent, { force: true, recursive: true });
  }
};
const rawBundle = (
  entries: { name: string; content?: string; mode?: number }[]
) => {
  const chunks: Buffer[] = [];
  for (const entry of entries) {
    const content = Buffer.from(entry.content ?? "");
    const header = new Header({
      mode: entry.mode ?? 0o644,
      path: entry.name,
      size: content.length,
      type: "File",
    });
    header.encode();
    if (!header.block) {
      throw new Error("Missing tar header");
    }
    chunks.push(
      header.block,
      content,
      Buffer.alloc((512 - (content.length % 512)) % 512)
    );
  }
  chunks.push(Buffer.alloc(1024));
  const archive = gzipSync(Buffer.concat(chunks));
  return { archive, plan: { archive_sha256: checksum(archive) } };
};
describe("safe pinned scaffolds", () => {
  test("preserves files and refuses existing target", async () => {
    const { plan, archive } = await bundle({
      ".env.example": "PUBLIC=example",
      "README.md": "hello",
    });
    await temporary(async (target) => {
      expect(await applyBundle(plan, archive, target)).toBe(2);
      expect(await fs.readFile(path.join(target, "README.md"), "utf-8")).toBe(
        "hello"
      );
      await expect(applyBundle(plan, archive, target)).rejects.toThrow();
      expect(await fs.readFile(path.join(target, "README.md"), "utf-8")).toBe(
        "hello"
      );
    });
  });
  test("rejects corruption before creating the directory", async () => {
    const { plan, archive } = await bundle({ a: "hello" });
    await temporary(async (target) => {
      await expect(
        applyBundle(plan, Buffer.concat([archive, Buffer.from("bad")]), target)
      ).rejects.toThrow();
      expect(
        await fs.lstat(target).then(
          () => true,
          () => false
        )
      ).toBe(false);
    });
  });
  test("write and permission failures clean up and preserve the original failure", async () => {
    const { plan, archive } = await bundle({ a: "hello" });
    for (const operation of ["writeFile", "chmod"] as const) {
      await temporary(async (target) => {
        const failure = new Error("injected extraction failure");
        const io = {
          ...fs,
          [operation]: () => Promise.reject(failure),
        };
        await expect(applyBundle(plan, archive, target, io)).rejects.toBe(
          failure
        );
        expect(
          await fs.lstat(target).then(
            () => true,
            () => false
          )
        ).toBe(false);
      });
    }
  });
  test("rejects symlinks, git internals and multiple source roots", async () => {
    const parent = await fs.mkdtemp(path.join(os.tmpdir(), "hive-unsafe-"));
    try {
      await fs.mkdir(path.join(parent, "root"));
      await fs.symlink("/etc/passwd", path.join(parent, "root", "link"));
      const archive = Buffer.concat(
        await Array.fromAsync(create({ cwd: parent, gzip: true }, ["root"]))
      );
      await expect(
        verifyBundle({ archive_sha256: checksum(archive) }, archive)
      ).rejects.toThrow();
      const hidden = await bundle({ ".git/config": "unsafe" });
      await expect(verifyBundle(hidden.plan, hidden.archive)).rejects.toThrow();
      await fs.mkdir(path.join(parent, "other"));
      const multi = Buffer.concat(
        await Array.fromAsync(
          create({ cwd: parent, gzip: true }, ["root", "other"])
        )
      );
      await expect(
        verifyBundle({ archive_sha256: checksum(multi) }, multi)
      ).rejects.toThrow();
    } finally {
      await fs.rm(parent, { force: true, recursive: true });
    }
  });
  test("rejects traversal and duplicates before writing, and cleans conflicting file prefixes", async () => {
    for (const entries of [
      [{ name: "../escape" }],
      [{ name: "root/../escape" }],
      [{ name: "/root/file" }],
      [{ name: "root/a" }, { name: "root/a" }],
    ]) {
      const { plan, archive } = rawBundle(entries);
      await expect(verifyBundle(plan, archive)).rejects.toThrow();
    }
    const { plan, archive } = rawBundle([
      { content: "file", name: "root/a" },
      { content: "nested", name: "root/a/b" },
    ]);
    await temporary(async (target) => {
      await expect(applyBundle(plan, archive, target)).rejects.toThrow();
      expect(
        await fs.lstat(target).then(
          () => true,
          () => false
        )
      ).toBe(false);
    });
  });
  test("preserves executable modes and bounds expanded archives", async () => {
    const { plan, archive } = rawBundle([
      { content: "echo hello", mode: 0o755, name: "root/run.sh" },
    ]);
    await temporary(async (target) => {
      await applyBundle(plan, archive, target);
      const stats = await fs.stat(path.join(target, "run.sh"));
      expect(stats.mode % 8).toBe(5);
    });
    const expanded = gzipSync(Buffer.alloc(32 * 1024 * 1024 + 1));
    await expect(
      verifyBundle({ archive_sha256: checksum(expanded) }, expanded)
    ).rejects.toThrow();
  });
});
