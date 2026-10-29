import { fileURLToPath } from "node:url";
import { parseEnv } from "node:util";

const root = fileURLToPath(new URL("../../../../", import.meta.url));
const defaults = parseEnv(await Bun.file(`${root}/.env.development`).text());
const env = {
  ...defaults,
  ...process.env,
  BASE_SEPOLIA_RPC_URL:
    process.env.HIVE_LOCAL_RPC_URL ?? "http://127.0.0.1:8545",
  DODO_PAYMENTS_API_KEY: "",
  DODO_PAYMENTS_PRODUCT_ID: "",
  DODO_PAYMENTS_WEBHOOK_SECRET: "",
  HIVE_CONFIG_FILE: `${root}/.hive/local/hiveway.toml`,
  OPENAI_API_KEY: "",
  X402_FACILITATOR_URL: "http://127.0.0.1:4022",
};
// Local acceptance never loads live payment credentials from .env.local.
const child = Bun.spawn(["cargo", "run", "--locked", "--package", "hiveway"], {
  cwd: root,
  env,
  stderr: "inherit",
  stdout: "inherit",
});
for (const signal of ["SIGTERM", "SIGINT"] as const) {
  process.once(signal, () => child.kill(signal));
}
process.exitCode = await child.exited;
