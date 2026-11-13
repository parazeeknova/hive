import { fileURLToPath } from "node:url";

import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  agentRules: false,
  devIndicators: false,
  headers: () =>
    Promise.resolve([
      {
        headers: [{ key: "X-Robots-Tag", value: "noindex, nofollow" }],
        source: "/api/:path*",
      },
      {
        // OAuth consent must never render inside another site's frame.
        headers: [
          { key: "X-Frame-Options", value: "DENY" },
          { key: "Content-Security-Policy", value: "frame-ancestors 'none'" },
          { key: "X-Robots-Tag", value: "noindex, nofollow" },
        ],
        source: "/oauth/:path*",
      },
    ]),
  output: "standalone",
  outputFileTracingRoot: fileURLToPath(new URL("../..", import.meta.url)),
  poweredByHeader: false,
  serverExternalPackages: [
    "pino",
    "@opentelemetry/sdk-node",
    "@opentelemetry/sdk-logs",
    "@opentelemetry/instrumentation-undici",
  ],
};

export default nextConfig;
