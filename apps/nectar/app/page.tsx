import { context, propagation, trace } from "@opentelemetry/api";

import { logger } from "../lib/logger";

const Home = () =>
  trace.getTracer("nectar").startActiveSpan("hiveway.health", async (span) => {
    try {
      const headers: Record<string, string> = {};
      propagation.inject(context.active(), headers);
      const response = await fetch("http://127.0.0.1:3001/api/health", {
        cache: "no-store",
        headers,
        signal: AbortSignal.timeout(5000),
      });
      if (!response.ok) {
        throw new Error(`Hiveway returned ${response.status}`);
      }
      const health: unknown = await response.json();
      if (
        typeof health !== "object" ||
        health === null ||
        !("status" in health) ||
        health.status !== "ok"
      ) {
        throw new Error("Hiveway returned an invalid health response");
      }
      logger.info("Hiveway health request completed");
      return <main className="p-4">{health.status}</main>;
    } finally {
      span.end();
    }
  });

export default Home;
