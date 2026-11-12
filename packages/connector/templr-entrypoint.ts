import { mkdir, writeFile } from "node:fs/promises";

const dir = "/tmp/templr-connector";
const hive = process.env.HIVE_URL ?? "https://hiveway.itssingularity.com/mcp";
const upstream = process.env.UPSTREAM_URL;
const key = process.env.TEMPLR_HIVE_KEY;
const upstreamToken = process.env.TEMPLR_UPSTREAM_TOKEN;

if (!upstream || !key) {
  console.error("UPSTREAM_URL and TEMPLR_HIVE_KEY must both be set");
  process.exitCode = 1;
} else {
  const config = {
    hive,
    keyFile: `${dir}/hive-key`,
    name: "templr",
    transport: "http",
    url: upstream,
    ...(upstreamToken
      ? { headers: { Authorization: `Bearer ${upstreamToken}` } }
      : {}),
  };
  await mkdir(dir, { recursive: true });
  await writeFile(`${dir}/connector.json`, JSON.stringify(config));
  await writeFile(`${dir}/hive-key`, key);
  const child = Bun.spawn(
    ["bun", "packages/connector/src/index.ts", `${dir}/connector.json`],
    { stdio: ["inherit", "inherit", "inherit"] }
  );
  process.exitCode = (await child.exited) ?? 1;
}
