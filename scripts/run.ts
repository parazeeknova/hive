import { fileURLToPath } from "node:url";

export const root = fileURLToPath(new URL("../", import.meta.url));
export const readOnly =
  Boolean(process.env.CI) || process.argv.includes("--ci");

export const run = async (command: string[], cwd = root, required = true) => {
  console.log(`\n$ ${command.join(" ")}`);
  const child = Bun.spawn(command, {
    cwd,
    env: { ...process.env, NEXT_TELEMETRY_DISABLED: "1", NX_DAEMON: "false" },
    stderr: "inherit",
    stdin: "inherit",
    stdout: "inherit",
  });
  const code = await child.exited;
  if (required && code !== 0) {
    throw new Error(`${command.join(" ")} failed (${code})`);
  }
  return code;
};

export const checkThenFix = async (check: string[], fix: string[]) => {
  const code = await run(check, root, false);
  if (code !== 0) {
    if (readOnly) {
      throw new Error(`${check.join(" ")} failed; CI does not modify files`);
    }
    await run(fix);
    await run(check);
  }
};
