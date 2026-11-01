# syntax=docker/dockerfile:1
FROM oven/bun:1.4.4@sha256:1d22548f177f4ff38e0c15baa3c2c46c6c2060ba2d1bc573bb4720b407548f1d
WORKDIR /app
COPY package.json bun.lock ./
COPY apps/nectar/package.json apps/nectar/package.json
COPY apps/hiveway/package.json apps/hiveway/package.json
COPY packages/local/package.json packages/local/package.json
COPY packages/connector/package.json packages/connector/package.json
RUN bun install --frozen-lockfile --ignore-scripts && mkdir /workspace && chown bun:bun /workspace
COPY --chown=bun:bun packages/connector/src packages/connector/src
USER bun
CMD ["bun", "packages/connector/src/index.ts", "/run/secrets/connector.json"]
