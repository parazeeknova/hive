import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../../../", import.meta.url));
const run = async (command: string[]) => {
  const child = Bun.spawn(command, {
    cwd: root,
    env: process.env,
    stderr: "inherit",
    stdout: "inherit",
  });
  if ((await child.exited) !== 0) {
    throw new Error(`Local infrastructure failed: ${command.join(" ")}`);
  }
};
await run([
  "docker",
  "compose",
  "--profile",
  "payments",
  "up",
  "--detach",
  "--wait",
  "anvil",
]);
await run(["bun", "packages/local/src/chain/setup.ts"]);
await run([
  "docker",
  "compose",
  "--profile",
  "payments",
  "up",
  "--detach",
  "--build",
  "--wait",
  "facilitator",
]);
console.log(
  "Local payments ready. Start Hiveway with HIVE_CONFIG_FILE=.hive/local/hiveway.toml"
);
