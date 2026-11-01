import { expect, test } from "bun:test";
import { mkdtemp, mkdir, writeFile, symlink, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import {
  Directories,
  contains,
} from "../../packages/connector/src/directories";

test("directory approval is local, canonical, session scoped and excludes credentials", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "hive-dirs-"));
  const workspace = path.join(root, "workspace");
  const key = path.join(root, "key");
  await mkdir(workspace);
  await writeFile(key, "not-shared");
  const confirmations: string[] = [];
  const directories = new Directories(key, (directory) => {
    confirmations.push(directory);
    return true;
  });
  try {
    expect(await directories.allow(workspace)).toBe(true);
    expect(await directories.allow(workspace)).toBe(true);
    expect(confirmations).toEqual([workspace]);
    await symlink(root, path.join(workspace, "escape"));
    await expect(
      directories.allow(path.join(workspace, "escape"))
    ).rejects.toThrow("credential");
    expect(directories.allow(root)).rejects.toThrow("credential");
    const denied = new Directories(key, () => false);
    expect(await denied.allow(workspace)).toBe(false);
    expect(denied.approved.size).toBe(0);
    expect(contains(workspace, `${workspace}-other`)).toBe(false);
    expect(directories.allow("relative")).rejects.toThrow("absolute");
  } finally {
    await rm(root, { force: true, recursive: true });
  }
});
