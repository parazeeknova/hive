import { realpath, stat } from "node:fs/promises";
import path from "node:path";
import { createInterface } from "node:readline/promises";

export const contains = (root: string, candidate: string) => {
  const relative = path.relative(root, candidate);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) &&
      relative !== ".." &&
      !path.isAbsolute(relative))
  );
};
const terminalConfirm = async (directory: string) => {
  if (!process.stdin.isTTY || !process.stderr.isTTY) {
    return false;
  }
  const terminal = createInterface({
    input: process.stdin,
    output: process.stderr,
  });
  try {
    const answer = await terminal.question(
      `Allow Hive filesystem read/write access to ${JSON.stringify(directory)} for this connector session? Type yes within 20 seconds: `,
      { signal: AbortSignal.timeout(20_000) }
    );
    return answer.trim() === "yes";
  } catch {
    return false;
  } finally {
    terminal.close();
  }
};
export class Directories {
  readonly approved = new Set<string>();
  private readonly keyFile: string;
  private readonly confirm: (directory: string) => boolean | Promise<boolean>;
  constructor(
    keyFile: string,
    confirm: (directory: string) => boolean | Promise<boolean> = terminalConfirm
  ) {
    this.keyFile = keyFile;
    this.confirm = confirm;
  }
  async allow(directory: string) {
    if (!path.isAbsolute(directory)) {
      throw new Error("Request an absolute working directory");
    }
    const canonical = await realpath(directory);
    const info = await stat(canonical);
    if (!info.isDirectory()) {
      throw new Error("Not a directory");
    }
    if (contains(canonical, await realpath(this.keyFile))) {
      throw new Error(
        "A directory containing the Hive credential cannot be allowed"
      );
    }
    if ([...this.approved].some((root) => contains(root, canonical))) {
      return true;
    }
    if (!(await this.confirm(canonical))) {
      return false;
    }
    this.approved.add(canonical);
    return true;
  }
}
