import { spawn } from "node:child_process";
import { accessSync, constants, existsSync } from "node:fs";
import { parseEnv } from "node:util";

import { root, run } from "./run";

const defaults = parseEnv(
  await Bun.file(new URL("../.env.development", import.meta.url)).text()
);
const localFile = Bun.file(new URL("../.env.local", import.meta.url));
const local = (await localFile.exists())
  ? parseEnv(await localFile.text())
  : {};
const inherited = { ...process.env };
Object.assign(process.env, defaults, local, inherited);
const token = Buffer.from(
  `${process.env.ZO_ROOT_USER_EMAIL}:${process.env.ZO_ROOT_USER_PASSWORD}`
).toString("base64");
process.env.OTEL_EXPORTER_OTLP_HEADERS ??= `Authorization=Basic%20${token},stream-name=hive`;
// Prefer a usable system Podman API, then a rootless API when Docker is absent.
const runtimeDir =
  process.env.XDG_RUNTIME_DIR ?? `/run/user/${process.getuid?.()}`;
if (!process.env.DOCKER_HOST && !existsSync("/var/run/docker.sock")) {
  const socket = [
    "/run/podman/podman.sock",
    `${runtimeDir}/podman/podman.sock`,
  ].find((candidate) => {
    try {
      accessSync(candidate, constants.R_OK);
      accessSync(candidate, constants.W_OK);
      return true;
    } catch {
      return false;
    }
  });
  if (socket) {
    process.env.DOCKER_HOST = `unix://${socket}`;
  }
}
// Podman exposes a native image builder through its Docker-compatible API.
if (process.env.DOCKER_HOST?.includes("/podman/")) {
  process.env.DOCKER_BUILDKIT ??= "0";
}
await run([
  "docker",
  "compose",
  "--env-file",
  ".env.development",
  "build",
  "--pull",
  "postgres",
  "redis",
]);
await run([
  "docker",
  "compose",
  "--env-file",
  ".env.development",
  "up",
  "--detach",
  "--wait",
  "--wait-timeout",
  "120",
]);
const deadline = Date.now() + 120_000;
let ready = false;
while (Date.now() < deadline) {
  try {
    // oxlint-disable-next-line no-await-in-loop -- Readiness polling must be sequential.
    const response = await fetch("http://127.0.0.1:5080/healthz", {
      signal: AbortSignal.timeout(2000),
    });
    if (response.ok) {
      ready = true;
      break;
    }
  } catch {
    // OpenObserve may still be starting after its container is running.
  }
  // oxlint-disable-next-line no-await-in-loop -- Wait between readiness probes.
  await Bun.sleep(500);
}
if (!ready) {
  throw new Error(
    "OpenObserve did not become healthy; inspect docker compose logs openobserve"
  );
}

// Compile first so the first page request can reach the API while Next starts.
await run(["cargo", "build", "--locked", "--package", "hiveway"]);

const child = spawn("lerna", ["run", "dev", "--parallel", "--stream"], {
  cwd: root,
  detached: true,
  env: { ...process.env, NEXT_TELEMETRY_DISABLED: "1", NX_DAEMON: "false" },
  stdio: "inherit",
});

const stop = (signal: NodeJS.Signals) => {
  if (child.pid !== undefined) {
    try {
      process.kill(-child.pid, signal);
    } catch (error) {
      if (
        !(error instanceof Error && "code" in error && error.code === "ESRCH")
      ) {
        throw error;
      }
    }
  }
};

process.once("SIGINT", () => stop("SIGINT"));
process.once("SIGTERM", () => stop("SIGTERM"));
child.once("error", (error) => {
  console.error(error.message);
  process.exitCode = 1;
});
child.once("exit", (code, signal) => {
  stop("SIGTERM");
  process.exitCode = code ?? (signal === "SIGINT" ? 130 : 1);
});
