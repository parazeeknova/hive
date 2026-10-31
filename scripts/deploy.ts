/* oxlint-disable no-await-in-loop -- Filesystem writes, queue polling and API-first deployment must run in order. */
/** Deploy immutable GHCR images through Dokploy, then verify both public revisions. */
import { appendFile } from "node:fs/promises";

export interface Runtime {
  fetch: typeof fetch;
  now: () => number;
  sleep: (milliseconds: number) => Promise<unknown>;
}
const runtime: Runtime = {
  fetch: globalThis.fetch,
  now: () => performance.now(),
  sleep: Bun.sleep,
};
const required = (name: string) => {
  const value = process.env[name]?.trim();
  if (!value) {
    throw new Error(`Missing ${name}`);
  }
  return value;
};
export const requestJson = async <T>(
  url: string,
  options: { body?: unknown; key?: string; allowEmpty?: boolean } = {},
  io = runtime
): Promise<T> => {
  const response = await io.fetch(url, {
    ...(options.body === undefined
      ? {}
      : { body: JSON.stringify(options.body), method: "POST" }),
    headers: {
      Accept: "application/json",
      "Cache-Control": "no-cache",
      "Content-Type": "application/json",
      "User-Agent": "Hive-deploy/1.0",
      ...(options.key ? { "x-api-key": options.key } : {}),
    },
    redirect: "error",
    signal: AbortSignal.timeout(30_000),
  });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} from ${new URL(url).pathname}`);
  }
  const payload = await response.text();
  if (options.allowEmpty && !payload.trim()) {
    return null as T;
  }
  return JSON.parse(payload) as T;
};
interface Deployment {
  deploymentId: string;
  title?: string;
  status?: string;
}
export class Dokploy {
  private url: string;
  private key: string;
  private io: Runtime;
  constructor(url: string, key: string, io = runtime) {
    const parsed = new URL(url);
    if (
      parsed.protocol !== "https:" ||
      parsed.username ||
      parsed.password ||
      parsed.search ||
      parsed.hash
    ) {
      throw new Error("Dokploy requires a credential-free HTTPS URL");
    }
    this.url = url.replace(/\/$/u, "");
    this.key = key;
    this.io = io;
  }
  api<T = unknown>(
    endpoint: string,
    body?: unknown,
    query?: Record<string, string>
  ): Promise<T> {
    const url = `${this.url}/api/${endpoint}${query ? `?${new URLSearchParams(query)}` : ""}`;
    return requestJson<T>(
      url,
      {
        allowEmpty: body !== undefined,
        ...(body === undefined ? {} : { body }),
        key: this.key,
      },
      this.io
    );
  }
  image(applicationId: string, image: string) {
    return this.api("application.saveDockerProvider", {
      applicationId,
      dockerImage: image,
      password: null,
      registryUrl: "ghcr.io",
      username: null,
    });
  }
  async deploy(applicationId: string, image: string, title: string) {
    const previous = await this.api<Deployment[]>("deployment.all", undefined, {
      applicationId,
    });
    const existing = new Set(previous.map((item) => item.deploymentId));
    await this.image(applicationId, image);
    await this.api("application.deploy", { applicationId, title });
    const deadline = this.io.now() + 600_000;
    while (this.io.now() < deadline) {
      const deployments = await this.api<Deployment[]>(
        "deployment.all",
        undefined,
        { applicationId }
      );
      const current = deployments.find(
        (item) => !existing.has(item.deploymentId) && item.title === title
      );
      if (current?.status === "done") {
        return;
      }
      if (
        current?.status &&
        ["error", "cancelled", "canceled"].includes(current.status)
      ) {
        throw new Error(
          `Dokploy deployment ${title} failed: ${current.status}`
        );
      }
      await this.io.sleep(10_000);
    }
    throw new Error(`Timed out waiting for deployment ${title}`);
  }
}
interface Health {
  status?: string;
  nectar?: string;
  revision?: string;
  hiveway?: { revision?: string };
}
export const verifyHealth = async (
  origin: string,
  revision: string,
  frontend = false,
  timeout = 240_000,
  io = runtime
) => {
  const deadline = io.now() + timeout;
  let lastError: unknown;
  while (io.now() < deadline) {
    try {
      const body = await requestJson<Health>(
        `${origin.replace(/\/$/u, "")}/api/health`,
        {},
        io
      );
      if (
        (frontend ? body.nectar : body.status) === "ok" &&
        body.revision === revision
      ) {
        if (frontend && body.hiveway?.revision !== revision) {
          throw new Error(
            "Frontend is connected to a different backend release"
          );
        }
        if (!frontend) {
          const ready = await requestJson<Health>(
            `${origin.replace(/\/$/u, "")}/api/ready`,
            {},
            io
          );
          if (ready.status !== "ok") {
            throw new Error("Backend dependencies are not ready");
          }
        }
        return;
      }
      throw new Error(
        `Unexpected health status or revision (expected ${revision}, received ${body.revision ?? "unknown"})`
      );
    } catch (error_) {
      lastError = error_;
    }
    await io.sleep(10_000);
  }
  throw new Error(
    `Health or revision verification failed for ${origin}: ${lastError instanceof Error ? lastError.message : "no valid response"}`
  );
};
export const immutableImage = (image: string, app: string) => {
  if (
    !/^(?:nectar|hiveway)$/u.test(app) ||
    !new RegExp(
      `^ghcr\\.io/[a-z0-9_.-]+/hive-${app}@sha256:[0-9a-f]{64}$`,
      "u"
    ).test(image)
  ) {
    throw new Error(`Expected an immutable GHCR digest for ${app}`);
  }
  return image;
};
export const verifyPublicImage = async (image: string, io = runtime) => {
  const [repository, digest] = image.replace(/^ghcr\.io\//u, "").split("@");
  try {
    const url = `https://ghcr.io/token?${new URLSearchParams({ scope: `repository:${repository}:pull`, service: "ghcr.io" })}`;
    const { token } = await requestJson<{ token: string }>(url, {}, io);
    if (!token) {
      throw new Error("Missing anonymous pull token");
    }
    const response = await io.fetch(
      `https://ghcr.io/v2/${repository}/manifests/${digest}`,
      {
        headers: {
          Accept:
            "application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.v2+json",
          Authorization: `Bearer ${token}`,
        },
        method: "HEAD",
        redirect: "error",
        signal: AbortSignal.timeout(30_000),
      }
    );
    if (!response.ok) {
      throw new Error("Anonymous pull failed");
    }
  } catch {
    throw new Error(
      `${repository} must be Public in GitHub package settings before deployment`
    );
  }
};
const main = async () => {
  const revision = required("RELEASE_REVISION");
  if (!/^[0-9a-f]{40}$/u.test(revision)) {
    throw new Error("Expected a full Git revision");
  }
  const client = new Dokploy(
    required("DOKPLOY_URL"),
    required("DOKPLOY_API_KEY")
  );
  const releases = ["hiveway", "nectar"].map((app) => ({
    app,
    id: required(`DOKPLOY_${app.toUpperCase()}_ID`),
    image: immutableImage(required(`${app.toUpperCase()}_IMAGE`), app),
    origin: required(`${app.toUpperCase()}_PUBLIC_URL`),
  }));
  for (const release of releases) {
    await verifyPublicImage(release.image);
  }
  for (const release of releases) {
    process.stdout.write(`Deploying ${release.app}: ${release.image}\n`);
    await client.deploy(
      release.id,
      release.image,
      `Release ${revision} (${release.app})`
    );
    await verifyHealth(release.origin, revision, release.app === "nectar");
    process.stdout.write(`Verified ${release.app} at ${revision}\n`);
  }
  if (process.env.GITHUB_STEP_SUMMARY) {
    await appendFile(
      process.env.GITHUB_STEP_SUMMARY,
      `Deployed and verified release \`${revision}\` on Hiveway and Nectar.\n`
    );
  }
};
if (import.meta.main) {
  await main();
}
