import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  agentRules: false,
  headers: () =>
    Promise.resolve([
      {
        headers: [{ key: "X-Robots-Tag", value: "noindex, nofollow" }],
        source: "/api/:path*",
      },
    ]),
  poweredByHeader: false,
  serverExternalPackages: [
    "pino",
    "@opentelemetry/sdk-node",
    "@opentelemetry/sdk-logs",
    "@opentelemetry/instrumentation-undici",
  ],
};

export default nextConfig;
