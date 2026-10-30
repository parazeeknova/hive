# Local agent acceptance

Use `devenv shell` for the commands below. These fixtures sign only on a loopback Anvil chain (31337) using its public test mnemonic. They never load production payment credentials.

Start PostgreSQL and Redis with `docker compose up --detach --wait postgres redis`. Start the isolated payment chain and facilitator with `bun packages/local/src/chain/start.ts`, then Hiveway with `bun packages/local/src/agents/serve.ts`.

Run the acceptance flows in order:

```sh
HIVE_DEMO_REVISION=$(git rev-parse HEAD) bun packages/local/src/agents/demo.ts
bun packages/local/src/agents/hosted-demo.ts
bun packages/local/src/agents/mcp-demo.ts
```

The first two flows check creator payouts per invocation on the local chain, dynamic artifact results, sales, and replay without another transfer. The MCP flow checks protocol negotiation, artifact upload, free publication, dynamic tool discovery, execution, replay, rejection of changed input, and receipt ownership. It measures 30 sequential searches and writes a one-hour buyer credential to `.hive/local/opencode-token` with mode 0600. That credential can read and invoke capabilities; it cannot publish.

## OpenCode

Configure the remote MCP URL `http://127.0.0.1:3001/mcp` with an `Authorization: Bearer <credential>` header. Use the scoped credential above for local tests only. OpenCode v2 uses `mcp.servers.hive`; its `opencode mcp add hive --url ... --header ... --global` command writes the configuration. Keep configuration containing bearer credentials private. Refresh the credential and configuration after it expires. Existing MCP connections may need `opencode reload`.

Check `opencode mcp list`, then run:

```sh
opencode run --model meta/muse-spark-1.3-contributor \
  'Use Hive MCP to search Free MCP echo, invoke one free echo with a new UUID idempotency_key, and repeat the same input and key to verify replay.'
```

Agents can call Hive during their own tasks. Hosted artifacts are immutable, pure JSON-in/JSON-out Wasm with CPU, memory, input and output limits. They cannot access files or networks. Server-side workflows currently compose up to four owned repository-analysis capabilities; they do not call arbitrary external agents. Free invocations produce a receipt without fabricating a blockchain settlement or creator earnings. Paid invocations require a client wallet signature per quoted call.

## Measured acceptance (2026-10-30)

Meta Muse Spark 1.3 Contributor in OpenCode successfully searched, invoked a free echo artifact, and replayed the same invocation ID and result. Paid Wasm calls with two different inputs transferred 20,000 local token units total; the repository-analysis call transferred 10,000. All replays preserved balances.

With 10,000 disposable catalog entries on this machine, the indexed search run measured 25.35 ms for a cold HTTP request. A 64-request burst at concurrency 8 measured 4.50 ms median, 5.80 ms p95, 8.15 ms maximum, approximately 1,435 requests/second during that short burst. Thirty sequential MCP searches measured 3.50 ms median, 5.05 ms p95, and 22.75 ms maximum. These are loopback development measurements, not sustained capacity or an Internet latency guarantee. The default peer rate limit still applies. The tests do not establish a speedup against another agent workflow.

Rust unit and PostgreSQL/security integration tests also run against PostgreSQL 18.6. Dodo checkout/refund behavior uses integration fixtures in test mode. Real provider payments and a deployed public Hive service require separate acceptance.

On this machine the Compose facilitator build failed because the Podman host referenced a missing `catatonit` executable. Running `bun packages/local/src/payments/facilitator.ts` directly allowed the local chain tests to continue. The pinned PostgreSQL 18.6 container started and passed integration tests.
