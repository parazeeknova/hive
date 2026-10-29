import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  agentRules: false,
  poweredByHeader: false,
  rewrites() {
    return Promise.resolve([
      {
        destination: "http://127.0.0.1:3001/api/:path*",
        source: "/api/:path*",
      },
    ]);
  },
  serverExternalPackages: [
    "pino",
    "@opentelemetry/sdk-node",
    "@opentelemetry/sdk-logs",
    "@opentelemetry/instrumentation-undici",
  ],
};

export default nextConfig;
