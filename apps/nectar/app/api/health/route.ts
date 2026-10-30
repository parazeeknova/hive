import { context, propagation, trace } from "@opentelemetry/api";

import { logger } from "../../../lib/logger";

export const dynamic = "force-dynamic";

export const GET = () =>
  trace.getTracer("nectar").startActiveSpan("hiveway.health", async (span) => {
    try {
      const headers: Record<string, string> = {};
      propagation.inject(context.active(), headers);
      const origin =
        process.env.HIVEWAY_ORIGIN ||
        process.env.NEXT_PUBLIC_HIVEWAY_ORIGIN ||
        "http://127.0.0.1:3001";
      const response = await fetch(`${origin.replace(/\/$/u, "")}/api/health`, {
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
      return Response.json({
        hiveway: health,
        nectar: "ok",
        revision: process.env.HIVE_BUILD_REVISION ?? "development",
      });
    } finally {
      span.end();
    }
  });
