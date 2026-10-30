# syntax=docker/dockerfile:1
FROM oven/bun:1.4.4@sha256:1d22548f177f4ff38e0c15baa3c2c46c6c2060ba2d1bc573bb4720b407548f1d AS bun
FROM node:24-bookworm-slim@sha256:c86ecead9e55a5210ac1727de38a4ca840bf03dcaf34dc9749addbf383b1920c AS dependencies
COPY --from=bun /usr/local/bin/bun /usr/local/bin/bun
WORKDIR /workspace
COPY package.json bun.lock ./
COPY apps/nectar/package.json apps/nectar/package.json
COPY apps/hiveway/package.json apps/hiveway/package.json
COPY packages/local/package.json packages/local/package.json
RUN bun install --frozen-lockfile --ignore-scripts

FROM dependencies AS build
ARG NEXT_PUBLIC_HIVEWAY_ORIGIN=https://hiveway.itssingularity.com
ENV NEXT_PUBLIC_HIVEWAY_ORIGIN=$NEXT_PUBLIC_HIVEWAY_ORIGIN
ENV NEXT_TELEMETRY_DISABLED=1
COPY apps/nectar apps/nectar
RUN bun --cwd apps/nectar next build

FROM node:24-bookworm-slim@sha256:c86ecead9e55a5210ac1727de38a4ca840bf03dcaf34dc9749addbf383b1920c AS runtime
ARG HIVE_BUILD_REVISION=development
ENV HIVE_BUILD_REVISION=$HIVE_BUILD_REVISION
ENV NODE_ENV=production NEXT_TELEMETRY_DISABLED=1 HOSTNAME=0.0.0.0 PORT=3000
WORKDIR /app
COPY --from=build --chown=node:node /workspace/apps/nectar/.next/standalone ./
COPY --from=build --chown=node:node /workspace/apps/nectar/.next/static ./apps/nectar/.next/static
COPY --from=build --chown=node:node /workspace/apps/nectar/public ./apps/nectar/public
USER node
EXPOSE 3000
HEALTHCHECK --interval=15s --timeout=5s --start-period=60s --retries=4 CMD node -e "fetch('http://127.0.0.1:3000/api/health').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"
CMD ["node", "apps/nectar/server.js"]
