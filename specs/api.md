# Hiveway API contract for Nectar

This is the frontend integration contract for HIVE's marketplace. It describes implemented behavior on `dev`; changes will be added here as backend work lands. The generated schema is available at `GET /api/openapi.json`. Backend source and contract tests take precedence if an example and schema differ.

## Connection and conventions

- Local API origin: `http://127.0.0.1:3001`. Local Nectar origin: `http://127.0.0.1:3000`.
- Configure these origins in the frontend environment. Hiveway accepts the exact configured frontend origin, including scheme and port. `localhost` and `127.0.0.1` are different origins.
- JSON requests use `Content-Type: application/json`. Protected requests use exactly one `Authorization: Bearer <token>` header. There is no password login, cookie session, or refresh token endpoint.
- IDs are UUID strings. Timestamps are Unix seconds. Nullable fields are returned as `null`.
- Prices and credential budgets are **decimal strings in atomic payment units**, currently six decimals: `"10000"` means 0.01 USDC; `"1000000"` means 1 USDC. Use `bigint`/decimal arithmetic, never floating-point arithmetic for balances.
- x402 objects use camelCase; ordinary Hiveway DTOs use snake_case.
- Parse the body even on `202`, `402`, and `409`: those can contain normal payment requirements or an invocation receipt.
- Request DTOs reject unknown fields. Do not submit server-owned properties such as `creator_wallet`, `active`, `spent`, or `state`.
- The current APIs use JSON polling, not SSE or WebSockets.

## Endpoint map

| Method | Path | Credential/scope | Response |
| --- | --- | --- | --- |
| GET | `/api/health` | None | `{ "status": "ok" }` |
| GET | `/api/ready` | None | `{ "status": "ok", "postgres": "ok", "redis": "ok" }` or 503 |
| GET | `/api/config` | None | Public network, feature flags, origins, and limits |
| GET | `/api/openapi.json` | None | OpenAPI schema |
| POST | `/v1/auth/challenges` | None | Wallet challenge |
| POST | `/v1/auth/verify` | None | Issued/revoked credential |
| GET | `/v1/credentials/me` | Any valid credential | Credential and budget |
| GET | `/v1/capabilities?q=...` | `capabilities:read` | Array of active capabilities |
| GET | `/v1/capabilities/page` | `capabilities:read` | Filtered cursor page |
| GET | `/v1/capabilities/{id}` | `capabilities:read` | Pinned capability, including inactive listings |
| POST | `/v1/capabilities/{id}/availability` | Publishing scope for its runtime | Owned listing availability |
| POST | `/v1/artifacts` | `capabilities:publish` | Validated immutable artifact metadata |
| GET | `/v1/artifacts/{id}` | `capabilities:read` | Artifact metadata and schemas |
| POST | `/v1/capabilities` | `capabilities:publish` | Created capability, HTTP 200 |
| POST | `/v1/workflows` | `workflows:publish` | Created workflow capability, HTTP 200 |
| POST | `/v1/quotes` | `capabilities:invoke` | Quote, HTTP 201; idempotent replay is 200 |
| POST | `/v1/invocations/{id}` | `capabilities:invoke` | 402 payment requirements or receipt |
| GET | `/v1/invocations/{id}` | `receipts:read` | Owned receipt |
| POST | `/v1/invocations/{id}/reconcile` | `receipts:read` | Refreshed owned receipt |
| POST | `/v1/invocations/{id}/checkout` | `capabilities:invoke` | Dodo test checkout |
| POST | `/v1/invocations/{id}/checkout/recover` | `receipts:read` | Provider-verified recovery receipt |
| GET | `/v1/invocations/{id}/refund` | `receipts:read` | Owned refund operation |
| POST | `/v1/invocations/{id}/refund` | `receipts:read` | Request/reconcile owned refund |
| GET | `/v1/activity` | `receipts:read` | Recent owned invocations and budget |
| GET | `/v1/sales` | `sales:read` | Recent sales for this credential's wallet |
| POST | `/mcp` | Valid credential; tools enforce their scopes | JSON-RPC MCP gateway |

`POST /v1/webhooks/dodo` is provider-to-backend only. The frontend must never call it or receive its signing secret.

### Public integration configuration

Fetch `GET /api/config` before configuring wallet payments or upload controls. It returns:

```ts
interface PublicConfig {
  version: string;
  public_url: string;
  auth_origin: string;
  payment: {
    network: string;
    chain_id: number;
    asset: string;
    decimals: number;
    name: string;
    version: string;
    settlement_finality: "finalized" | "latest";
    creator_payouts: "direct_x402";
  };
  features: { dodo_test_checkout: boolean; wasm_artifacts: boolean };
  limits: {
    request_body_bytes: number;
    artifact_bytes: number;
    input_bytes: number;
    output_bytes: number;
    schema_bytes: number;
    json_depth: number;
    json_nodes: number;
    collection_items: number;
  };
}
```

The response contains no provider credentials. `latest` settlement is restricted to loopback development RPCs; public deployments require finalized-chain evidence. Read payment amounts from each quote even after reading this configuration.

## 1. Wallet login and scoped credentials

Connect an EVM wallet. Request a challenge, ask the connected wallet to sign **the exact returned message** using `signMessage`/`personal_sign`, then verify that signature. Wallet keys remain in the wallet. This message signature is separate from the typed-data signature used to pay for an invocation.

### Request a credential

`POST /v1/auth/challenges`:

```json
{
  "wallet": "0x1111111111111111111111111111111111111111",
  "action": {
    "type": "issue",
    "policy": {
      "name": "Nectar session",
      "scopes": ["capabilities:read", "capabilities:invoke", "receipts:read"],
      "per_call_limit": "100000",
      "total_limit": "1000000",
      "expires_at": 2000000000
    }
  }
}
```

Replace `expires_at` with a future time no more than **24 hours** away. `per_call_limit` must not exceed `total_limit`; both must be positive decimal strings no greater than `"1000000000"`. Scope names must be unique. Available scopes:

| Scope                  | Purpose                                    |
| ---------------------- | ------------------------------------------ |
| `capabilities:read`    | Browse/discover tools                      |
| `capabilities:publish` | Publish creator capabilities               |
| `capabilities:invoke`  | Quote/invoke any supported capability      |
| `repository:analyze`   | Legacy built-in repository invocation only |
| `receipts:read`        | Receipts, activity, refund operations      |
| `sales:read`           | Creator sales                              |
| `workflows:publish`    | Publish workflow bundles                   |

Use the minimum scopes needed for the current session. Creator sessions can add publishing, workflows, and sales scopes. Credential names have a maximum of 80 bytes.

Challenge response:

```ts
interface WalletChallenge {
  id: string;
  message: string;
  expires_at: number;
}
```

The challenge expires after five minutes and is single-use. The server limits challenge issuance to 10 requests per socket peer IP per minute. Verify the message's domain, wallet, policy, and expiry in the wallet UI; do not rewrite its whitespace.

`POST /v1/auth/verify`:

```json
{
  "challenge_id": "<challenge UUID>",
  "signature": "0x<personal-sign signature>"
}
```

Response:

```ts
interface AuthResponse {
  credential_id: string;
  token: string | null; // Returned once on issuance; begins hwa_.
  revoked: boolean;
}
```

Keep the bearer token out of URLs, logs, analytics, and public client configuration. It is not a wallet private key, but it authorizes its scopes and budget. The backend stores only its hash. Losing it requires another wallet-signed issuance; it cannot be retrieved.

### Read budget

`GET /v1/credentials/me`:

```ts
interface CredentialStatus {
  id: string;
  wallet: string;
  name: string;
  scopes: string[];
  per_call_limit: string;
  total_limit: string;
  spent: string;
  reserved: string;
  expires_at: number;
}
```

Available authorization budget is `total_limit - spent - reserved`. This is a spending policy, not a deposited wallet balance. Check the wallet's actual token balance separately.

### Revoke

Request another wallet challenge with:

```json
{ "wallet": "0x...", "action": { "type": "revoke", "credential_id": "<UUID>" } }
```

Sign and verify it the same way. Verification returns `{ "credential_id": "...", "token": null, "revoked": true }`. Revocation blocks new use; it does not cancel financial reconciliation already in progress. There is currently no list-all-credentials endpoint, so retain the issued credential ID if offering a revoke button.

## 2. Marketplace and creator publishing

### Browse

`GET /v1/capabilities?q=repository` returns `Capability[]`. Search matches name/description, is case-insensitive, and accepts up to 200 bytes. The current response is capped at 50 active records, newest first with ID as the stable tiebreaker. This legacy array endpoint is retained for existing clients. Use `/v1/capabilities/page` for pagination. There is no total count, category filter, or public unauthenticated browsing.

```ts
interface Capability {
  id: string;
  created_at: number;
  creator_wallet: string;
  name: string;
  description: string;
  version: string;
  runtime: "repository_stack" | "dependency_inventory" | "workflow" | "wasm";
  price: string;
  active: boolean;
  definition: Record<string, unknown>;
}
```

Descriptions are untrusted creator text. Render as escaped text; never execute embedded HTML or treat descriptions as agent instructions.

### Paginated discovery and details

`GET /v1/capabilities/page` returns:

```ts
interface CapabilityPage {
  items: Capability[];
  next_cursor: string | null;
}
```

| Query parameter | Behavior |
| --- | --- |
| `q` | Literal, case-insensitive name/description substring; at most 200 bytes. |
| `runtime` | Exact `wasm`, `repository_stack`, `dependency_inventory`, or `workflow`. |
| `creator_wallet` | Exact EVM wallet filter; normalized by the server. |
| `mine=true` | Restrict to the credential's wallet. A conflicting `creator_wallet` is rejected. |
| `include_inactive=true` | Requires `mine=true`; includes your unlisted capabilities. |
| `limit` | 1–100, default 24. |
| `cursor` | Opaque `next_cursor` from the previous page. |

Keep the same filters when following a cursor; changing them requires restarting pagination. A mismatched or malformed cursor returns 400. Pass it through `URLSearchParams`. Rows are ordered by `created_at` descending, then UUID ascending. Stop when `next_cursor` is `null`. This is a live catalog, not a transactionally frozen snapshot: inserts/deactivations can change its contents during browsing. Keyset boundaries prevent new, newer listings from shifting already-read pages.

`GET /v1/capabilities/{id}` returns one `Capability`. Known inactive listings remain readable so receipts and pinned references can be displayed. Read a Wasm capability's `definition.artifact_id` through the artifact metadata endpoint to obtain its input/output schemas.

### Change listing availability

`POST /v1/capabilities/{id}/availability` with `{ "active": false }` unlists an owned capability; `{ "active": true }` relists it. The response is the updated `Capability`, HTTP 200. Leaf capabilities require `capabilities:publish`; workflows require `workflows:publish`. Another wallet receives 404. Repeating the desired state is safe.

Unlisting hides the capability from active discovery and prevents new quotes. Existing quotes retain their immutable capability, price, and payment requirements; retrying their original quote request/idempotency key still returns the same quote. Existing invocations and receipts continue normally. Unlisting a leaf does not rewrite already-published workflow bundles that pin it; unlist those workflows separately if needed. Availability changes do not edit code, schemas, prices, names, or versions.

### Publish a built-in capability

`POST /v1/capabilities`:

```json
{
  "name": "Repository stack report",
  "description": "Analyze the declared technologies of a public repository revision.",
  "runtime": "repository_stack",
  "price": "10000"
}
```

Supported publishable leaf runtimes: `repository_stack`, `dependency_inventory`, and `wasm`. Name: nonempty, maximum 80 bytes. Description: nonempty, maximum 1,000 bytes. Price: `"1"` through `"1000000000"`. The creator wallet comes from the credential. Version is currently `"1"`; the response is the created `Capability`.

For a `wasm` runtime, upload an artifact first and include its `artifact_id` when publishing. Built-in runtimes reject an `artifact_id`. Use the availability route to unlist/relist a capability. There are currently no edit, deletion, or version-management endpoints.

### Upload and publish a hosted Wasm capability

`POST /v1/artifacts` with `capabilities:publish`:

```json
{
  "wasm_base64": "<base64 binary Wasm module>",
  "input_schema": {
    "type": "object",
    "required": ["message"],
    "properties": { "message": { "type": "string" } },
    "additionalProperties": false
  },
  "output_schema": { "type": "object" }
}
```

Both schemas must describe root objects using the supported JSON Schema 2020-12 subset. External/local references, `$id`, branching combinators (`allOf`, `anyOf`, `oneOf`, `not`, `if`/`then`/`else`, `dependentSchemas`), and structural depth above 12 are rejected. The default maximum schema size is 4,096 bytes each. Read configured limits from `/api/config`. The total JSON request, including base64 expansion, must also fit its request-body limit. Invocation input/output structures are independently bounded: defaults are depth 16, 4,096 JSON value nodes, and 256 entries per array/object. These limits apply even when a creator schema allows larger collections.

HTTP 200 returns an immutable artifact; retrying identical bytes and schemas under the same wallet returns the same ID:

```ts
interface Artifact {
  id: string;
  owner_wallet: string;
  sha256: string; // Fingerprint of code plus canonical schemas, not just raw Wasm bytes.
  byte_length: number;
  abi_version: 1;
  verification: "sandbox_validated";
  input_schema: Record<string, unknown>;
  output_schema: Record<string, unknown>;
  created_at: number;
}
```

`GET /v1/artifacts/{id}` returns the same metadata with `capabilities:read`; it never exposes executable bytes. A publisher can only reference artifacts owned by its wallet:

```json
{
  "name": "Hosted JSON tool",
  "description": "Processes the current request using a pure hosted Wasm module.",
  "runtime": "wasm",
  "artifact_id": "<owned artifact UUID>",
  "price": "10000"
}
```

The capability's server-owned `definition` contains `artifact_id` and `abi_version`. For invocation, supply any JSON object matching its artifact's input schema in the quote's `input`; the successful result matches its output schema. Use `capabilities:invoke`; a legacy `repository:analyze` credential cannot invoke Wasm. Each execution receives the current input in a fresh, fuel-metered instance. This is dynamic computation, not a cached report or static hosted file.

For creator tooling, ABI version 1 requires exports `memory`, `alloc(i32) -> i32`, and `run(i32, i32) -> i64`. Input/output are UTF-8 JSON. `run` returns `(output_pointer << 32) | output_length`. Modules have no host imports, WASI, network, filesystem, environment, or clocks; start functions are forbidden. Fuel, memory, output size, and concurrent instances are bounded. `sandbox_validated` means ABI/isolation validation, not independent business-quality verification. A runtime trap or schema failure occurs before x402 settlement. Arbitrary containers, external network APIs, and GitHub source deployment are not supported by this upload route.

### Publish a workflow bundle

`POST /v1/workflows`:

```json
{
  "name": "Repository overview",
  "description": "Stack and dependency reports for the same repository revision.",
  "steps": ["<capability UUID>", "<another capability UUID>"],
  "price": "20000"
}
```

There must be 1–4 distinct, active built-in repository leaf capabilities, all owned by the publishing wallet. Wasm steps are not supported in current repository bundles. Nested workflows and cross-creator steps are currently rejected. The response is a `Capability` with runtime `workflow` and `definition.steps`. Current bundles run their reports for the same input; arbitrary dataflow/conditional steps are planned.

## 3. Quote and execute

Built-in repository runtimes and repository workflow bundles accept:

```ts
interface RepositoryInput {
  repository_url: string; // https://github.com/<owner>/<repo>, public repositories only.
  revision: string; // Exactly 40 hexadecimal characters: a full immutable commit SHA.
}
```

Branches/tags, non-GitHub hosts, credentials in URLs, query strings, and private repositories are rejected. The frontend should ask for a commit SHA; the backend currently has no branch-resolution endpoint.

### Reserve a quote

`POST /v1/quotes`, with `Idempotency-Key: <new UUID>`:

```json
{
  "capability_id": "<UUID>",
  "input": {
    "repository_url": "https://github.com/owner/repo",
    "revision": "<full SHA>"
  }
}
```

Persist that idempotency key across retries of the **same request**. A changed input needs a new key. A duplicate returns the original quote; changed input with the same key returns 409. Quotes reserve the credential budget immediately and expire after 120 seconds. An expired reservation is cleaned up during a subsequent quote flow; refreshing the credential alone is not a quote-cleanup operation.

```ts
interface Quote {
  id: string; // Invocation ID.
  expires_at: number;
  payment: PaymentRequired;
}
interface PaymentRequired {
  x402Version: 2;
  resource: { url: string; description: string; mimeType: string };
  accepts: PaymentRequirements[];
}
interface PaymentRequirements {
  scheme: "exact";
  network: string; // e.g. eip155:84532; read this value, do not hard-code it.
  amount: string;
  asset: string; // Token contract; changes for isolated development chains.
  payTo: string; // Creator wallet.
  maxTimeoutSeconds: number;
  extra: { name: string; version: string };
}
```

### x402: real creator payments

1. Use the returned `network`, `asset`, `amount`, `payTo`, and domain metadata. Confirm them with the wallet/user's budget policy.
2. Use the x402 EVM client to create a signed payment payload with the buyer's connected wallet. The wallet owner must match the credential's wallet.
3. Encode the payload as base64 JSON and send `POST /v1/invocations/{id}` with `Payment-Signature: <base64 payload>` and the bearer token. The body is unused; `{}` is sufficient.
4. Read the returned receipt. A transfer may be submitted while the receipt remains pending finality. Poll `GET /v1/invocations/{id}` or request reconciliation.

The server supports x402 v2 `exact` EVM authorizations. Signing must preserve the quote's accepted payment requirements and resource. Do not hand-construct EIP-712 fields when the SDK can do it. For the locally deployed test token, configure an explicit SDK asset allowance and spending cap; it is not a recognized default token. Never import the public development mnemonic from `packages/local` into Nectar wallet code.

Calling the invocation without `Payment-Signature` returns HTTP 402 with a `PaymentRequired` body and a base64 `Payment-Required` header. A settled x402 receipt also exposes a base64 `Payment-Response` header. Both payment headers are exposed through CORS.

Real x402 payments go directly to the capability creator's wallet. There is no withdraw-from-Hive endpoint. A signature is payment authorization; a successful API request alone is not proof of settlement.

### Read or reconcile a receipt

`GET /v1/invocations/{id}` reads the stored receipt. `POST /v1/invocations/{id}/reconcile` re-checks the provider/chain and returns the updated receipt. Both are scoped to the **credential that created the invocation**, not all credentials belonging to the same wallet.

```ts
interface Receipt {
  id: string;
  capability_id: string;
  state: InvocationState;
  price: string;
  result: unknown | null; // Only disclosed when state is settled.
  settlement: Settlement | null; // Only disclosed when settled.
  error_code: string | null;
  execution_ms: number | null;
  result_bytes: number | null;
  payment_issue: string | null;
}
interface Settlement {
  success: boolean;
  transaction: string; // x402: EVM transaction hash; Dodo test: provider payment ID.
  network: string; // EVM CAIP-2 ID or dodo:test.
  payer: string | null;
  errorReason: string | null;
}
type InvocationState =
  | "quoted"
  | "verifying"
  | "executing"
  | "settling"
  | "settlement_unknown"
  | "settled"
  | "failed"
  | "expired"
  | "checkout_creating"
  | "checkout_review"
  | "checkout_pending"
  | "paid"
  | "execution_failed";
```

| State | Frontend behavior |
| --- | --- |
| `quoted` | Offer x402 payment, or Dodo test checkout when enabled. Show expiry. |
| `verifying`, `executing` | Show progress; keep the invocation ID. |
| `settling`, `settlement_unknown` | Show payment reconciliation/finality pending. Do not automatically pay again. |
| `checkout_creating` | Provider creation may be in flight; do not create another checkout. |
| `checkout_review` | Show operator review required; retain the existing invocation. |
| `checkout_pending` | Show/open the existing checkout URL, then poll/reconcile. |
| `paid` | Dodo test payment booked; worker execution is pending. |
| `settled` | Show output, receipt, and transaction/provider reference. |
| `failed`, `expired` | Show the error/expiry. A new attempt needs a new quote. |
| `execution_failed` | Show execution failure and refund status; it may already have been paid. |

Receipt HTTP status is 200 for `settled`, 409 for `failed`/`expired`/`execution_failed`, and 202 for other states. A 409 receipt has the `Receipt` shape, not necessarily `{ error }`. `settlement_unknown` may still have an `error_code` from an explicit facilitator rejection; this does not prove that no funds moved.

Poll with backoff, for example 2 seconds increasing to 10 seconds while the page is visible, and stop on settled/terminal states. Public-chain finality can take substantially longer than the request timeout. A transport timeout does not cancel a submitted invocation; fetch its receipt before offering any retry. Reserved funds stay held during uncertain settlement. A finalized proof of an expired, unused authorization can mark it failed and restore its reservation.

Current result shapes:

```ts
interface Dependency {
  manifest: string;
  ecosystem: string;
  name: string;
  requirement: string;
}
interface RepositoryReport {
  repository_url: string;
  revision: string;
  manifests: string[];
  technologies: string[];
  dependencies: Dependency[];
  analysis_limit_reached: boolean;
}
```

Use the response's `result` as the source of truth; runtimes may expose different projections. Workflow results contain repository identity and a `steps` array with each capability/version/runtime/result.

## 4. Dodo test checkouts and refunds

Dodo is **test-only**. It is not the production creator payout rail. Show its UI only when `GET /api/config` returns `features.dodo_test_checkout: true`.

`POST /v1/invocations/{id}/checkout` returns:

```ts
interface Checkout {
  invocation_id: string;
  state: InvocationState;
  checkout_id: string | null;
  checkout_url: string | null;
}
```

Only whole US cents are supported: the atomic price must be at least `"10000"` and divisible by 10,000. Open the returned provider URL; a browser redirect or success screen is not payment proof. Poll/reconcile the original invocation after checkout. The backend uses signed webhooks and canonical provider reads. Repeat calls return the existing checkout; they do not create another one.

For a lost checkout response with a known successful provider payment, `POST /v1/invocations/{id}/checkout/recover` with `{ "payment_id": "pay_..." }` returns a receipt after validating its canonical session, wallet, product, amount, and invocation. This is not a way to attach somebody else's payment.

A paid Dodo execution failure creates a durable full-refund operation. The owning agent can request/reconcile it using `POST /v1/invocations/{id}/refund`, or read it using `GET` at the same path:

```ts
interface RefundOperation {
  invocation_id: string;
  state:
    | "prepared"
    | "creating"
    | "pending"
    | "review"
    | "succeeded"
    | "failed";
  amount: string; // Atomic amount, not provider cents.
  refund_id: string | null;
  error_code: string | null;
}
```

Both refund endpoints return HTTP 200 for a readable operation; inspect `state`. A 404 can mean the operation does not exist yet. A successful capability invocation cannot be refunded through this route. The worker checks existing refund history, sends at most one creation POST per operation, and reconciles uncertain outcomes. Full confirmed refunds restore the credential's `spent` budget once. A partial refund does not restore the full budget.

`payment_issue` may be `refund_required`, `refund_creating`, `refund_pending`, `refund_review_required`, `refund_rejected`, `refunded`, `operator_review_required`, or a refund/dispute event type. Display unfamiliar values safely rather than assuming a closed enum. Never tell the customer a refund completed until its operation is `succeeded`.

## 5. Activity and creator sales

`GET /v1/activity` returns up to 100 invocations for this credential, newest first:

```ts
interface Activity {
  invocations: Array<{
    id: string;
    capability_id: string;
    state: InvocationState;
    price: string;
    payment_method: "x402" | "dodo" | null;
    payment_issue: string | null;
    execution_ms: number | null;
    result_bytes: number | null;
    created_at: number;
    completed_at: number | null;
  }>;
  budget: { limit: string; spent: string; reserved: string };
}
```

`GET /v1/sales` returns up to 100 wallet-level records, ordered by creation time and invocation ID descending:

```ts
interface Sale {
  invocation_id: string;
  creator_wallet: string;
  payer_wallet: string;
  amount: string;
  network: string;
  transaction_hash: string;
  created_at: number;
}
```

For x402, sales represent completed direct transfers. Dodo test entries are merchant accounting records, not money paid out to a creator wallet. Do not combine these into one “withdrawable balance.” There are no aggregate earnings, withdrawal, payout-request, or cursor-pagination endpoints yet.

## 6. Errors and request handling

Business errors normally use `{ "error": "machine_readable_code" }`. Axum JSON/query parsing errors can be text rather than that envelope, and admission failures can have an empty body. A fetch wrapper must handle JSON, text, and empty responses.

| HTTP | Meaning / handling |
| --- | --- |
| 400 / 422 | Invalid input, signature format, challenge, schema, or non-refundable invocation. Show actionable validation feedback. |
| 401 | Missing/expired/revoked credential or invalid signature. Reconnect/reissue through the wallet; no refresh endpoint exists. |
| 402 | Payment requirements; proceed to the wallet payment flow. |
| 403 | Scope, budget, Host/Origin, or ownership policy failure. Do not assume every 403 means insufficient budget. |
| 404 | Missing or inaccessible resource; invocation access is credential-specific. |
| 409 | Idempotency conflict, changed invocation state, or a terminal invocation receipt. Inspect the body shape. |
| 413 | Request body too large; default admission limit is 131,072 bytes. |
| 429 | Rate limit. Back off; do not repeatedly request wallet challenges. |
| 500 / 503 | Internal/provider/capacity failure. Preserve invocation and idempotency IDs before retrying. |
| 504 | HTTP wait expired; the handler may continue. Read the existing invocation before issuing another action. |

Useful current codes include `credential_required`, `credential_expired_or_revoked`, `challenge_expired_or_used`, `challenge_rate_limit`, `idempotency_key_reused_with_different_input`, `full_commit_sha_required`, `quote_not_payable`, `dodo_not_configured`, `dodo_product_not_configured`, `dodo_payment_binding_mismatch`, `invocation_not_refundable`, and `settlement_not_executed`. Treat unknown codes as displayable failures, not success.

Invalidate/refetch credential budget and activity after quoting, settlement, expiry reconciliation, or confirmed refund. Refetch creator sales after a settled invocation. Keep cached marketplace results visible during background refresh; credentials and payment receipts must not be cached publicly.

## 7. MCP and implementation boundaries

Nectar can use the REST endpoints directly. `/mcp` is for agent integrations; it uses scoped bearer credentials and JSON-RPC, not the REST error/response envelope. Tool results may contain `isError`. Do not use an HTTP 200 alone to decide that an MCP tool succeeded.

Implemented tool names include `discover_capabilities`, `search_capabilities`, `upload_artifact`, `publish_capability`, `compose_workflow`, `invoke_capability`, `dodo_checkout`, `get_receipt`, `refund_operation`, and `creator_sales`, depending on credential scopes. Published capabilities appear as dynamic `hive_<UUID-without-hyphens>` tools. `/mcp` supports the legacy initialization flow and the backend's newer metadata flow; use a matching MCP client rather than inventing transport headers.

### Planned contracts: do not ship frontend calls yet

Capability editing/version management, advanced workflow dataflow, and deployment/analytics APIs are being implemented. This document will gain their request/response contracts after the corresponding backend routes and tests land. No placeholder endpoint in this section is an implemented API.
