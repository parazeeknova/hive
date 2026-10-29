FROM oven/bun:1.4.4
WORKDIR /workspace
COPY package.json bun.lock bunfig.toml ./
COPY apps/hiveway/package.json apps/hiveway/package.json
COPY apps/nectar/package.json apps/nectar/package.json
COPY packages/local/package.json packages/local/package.json
RUN bun install --frozen-lockfile --ignore-scripts
COPY packages/local/src packages/local/src
COPY packages/local/contracts packages/local/contracts
USER bun
CMD ["bun", "packages/local/src/payments/facilitator.ts"]
