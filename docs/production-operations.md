# Production operation and verification

## Provider credentials

Set `HIVE_SECRETS_KEY` on the Hiveway backend to a persistent, randomly generated 32-byte value (`openssl rand -hex 32`). Store it in deployment secret storage, back it up securely, and preserve it across replicas and releases. Replacing this value does not re-encrypt existing provider credentials. `/api/config.features.provider_keys` must be true before accounts can save provider keys.

Provider credentials belong to each account. A deployment-wide encryption key enables storage; it does not supply Context7 or TinyFish API credentials. Save those separately in the account's Integrations settings.

## Shared MarkItDown, DuckDB and templr

Publishing the integration catalog does not start an upstream. Use the isolated worker and egress proxy in `docker/hive-local/compose.yaml` for MarkItDown and DuckDB; follow [Hive Local shared-service setup](../apps/hive-local/README.md#shared-services). The worker key must belong to the account configured by `HIVE_SHARED_OWNER_MARKITDOWN` and `HIVE_SHARED_OWNER_DUCKDB` on Hiveway. The compiled catalog defaults are deployment-specific; set these overrides explicitly for your operator account. Do not mount user project folders into this worker.

For deployments that promise shared services, set:

```text
HIVE_REQUIRED_SHARED_INTEGRATIONS=markitdown,duckdb
```

You can also require `templr` if the operator runs its connector. Unknown names and names with sharing disabled fail startup. `/api/ready` reports configured shared-service online status and returns 503 if a required service is offline. Its health check does not run upstream queries; verify a public document conversion and a read-only SQL query through an authenticated MCP client before release. Disconnected services are excluded from tool search, and discovery distinguishes an operator outage from personal Hive Local setup.

## Logs and traces

Hiveway emits JSON request and tool-completion logs at `info` and failed tool-completion logs at `warn`. The normal deployment filter must retain `hiveway=info`. Each HTTP request receives a server-generated `x-request-id`; HTTP spans include that ID, the method and matched route template. Tool spans include a call ID, credential ID and exact tool. Deferred workflow workers retain their parent span. Tool logs include semantic execution status even when the MCP response uses HTTP 200.

Configure the standard OTLP HTTP/protobuf variables for your collector. For OpenObserve, a deployment commonly uses:

```text
OTEL_EXPORTER_OTLP_ENDPOINT=https://YOUR-COLLECTOR/api/YOUR-ORG
OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
```

Supply `OTEL_EXPORTER_OTLP_HEADERS` using deployment secret storage, with the authorization required by your collector. Signal-specific `OTEL_EXPORTER_OTLP_LOGS_ENDPOINT`, `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` and corresponding header variables may override the shared endpoint. Confirm the collector's ingestion paths and permissions; a configured exporter is not proof of ingestion. Check both container stdout and the collector after an authenticated `search_tools` call. Match `request_id` and `call_id` instead of relying on message text alone.

Connector transport failures emit `upstream_request_failed` with the connection name, job ID, method and retry eligibility. The TypeScript connector reconnects and retries once only for tools explicitly classified as read-only by the operator catalog; it never retries application error envelopes or unknown/mutating tools. A successful transport recovery emits `upstream_read_recovered`. Bundled copies of the connector must be updated separately when their host application is deployed.

Arguments, results, authorization headers and provider keys are not added to the MCP activity ledger or these completion logs. Client identity is optional, bounded metadata; it is reported identity, not verified attribution.

## Activity and deferred execution

The Activity API returns existing payment `invocations` and new `tool_calls` with separate counts and one combined pagination cursor. Browser accounts see their account's calls; API keys see calls made with that credential. MCP rows show tool/provider, state, duration, safe error code, call ID and optional workflow/task IDs. They are not payment receipts. The dashboard merges and orders both event types and refreshes them normally.

Calls and workflow steps are recorded, including cache hits and failures. Background connector polling and completion bookkeeping are excluded from the user feed. Accepted asynchronous jobs and workflow runs are reconciled by the recovery worker without requiring client polling. Abandoned calls become `interrupted` with an unknown-outcome code; mutations are not retried. Activity retention uses `workflows.run_retention_seconds` in `apps/hiveway/config.toml`.

The new migration adds `mcp_activity`; it does not backfill historical calls that were never recorded. Apply it with the normal startup migration process. During a rolling upgrade, old replicas will not emit the new records. Inspect logs for activity write failures rather than assuming an empty feed means no calls ran.

## Repository prerequisites

Code tools require an indexed repository. From an approved checkout, use Hive Local's `sync_code_index` tool or the connector CLI:

```sh
bun packages/connector/src/code-index.ts --dir /absolute/path/to/repo \
  --hive https://YOUR-HIVE/mcp --key-file /absolute/path/outside/repo/hive-key
```

The credential needs `code:index`; readers need `code:read`. Index only the repository content intended for the account. New deployments and accounts legitimately start with empty indexes and memories.

## Release verification

Run the repository checks, including the database-backed regression with a disposable PostgreSQL database:

```sh
cargo test --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
bun test scripts/tests
bun run check-types --ci
HIVE_TEST_DATABASE_URL=postgres://... cargo test --locked -p hiveway \
  --test postgres --test security --test workflows --test mcp_readiness -- --ignored --test-threads=1
```

Before release, verify `/api/ready`, provider-key storage, a Context7 resolve/query, a TinyFish search/fetch, a combined workflow, and successful/failed rows in Activity. Check that matching completion events arrive in the production collector. Verify the same paths on the deployed image revision; local tests cannot establish production infrastructure availability.

## Discovery-led workflows

The calling agent chooses tools from `search_tools` and their advertised schemas, then supplies a plan to `run_workflow`. The runner executes that plan with the caller's credentials and connections. It does not generate plans with a separate model.

Result references infer dependencies automatically. For example, TinyFish search can supply `{{steps.search.json.results.0.url}}` to a later fetch; Context7 documentation can run alongside search; DuckDB can consume values from those results. Use `| sql` when inserting a result into a SQL string. Treat returned text as data to verify.

`for_each` passes each item as typed JSON, `when` skips unnecessary work, and `on_error=continue` records an optional failure while allowing independent work to finish. Required failures stop dependent steps. Tool calls and fan-out remain subject to the configured budgets; uncertain mutations are not replayed. A dry run validates graph/input structure, while execution checks current permissions and provider availability.

Activity finalization is preserved when completed connector jobs are removed, including connection-deletion cascades. Removed unfinished jobs become `interrupted` with an unknown-outcome code. The migration also closes existing orphaned accepted rows as interrupted; it cannot reconstruct results that were already deleted.
