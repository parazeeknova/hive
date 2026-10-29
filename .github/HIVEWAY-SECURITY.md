# Hiveway security checks

Scope: the current Hiveway source and owned local PostgreSQL/HTTP test harness. No third-party service was attacked. This is an engineering audit, not a guarantee against every attack or an independent penetration test.

## Changes

- Public repository analysis never sends a server GitHub credential. A broadly scoped token previously gave callers indirect access to private source manifests.
- Payment-provider capability checks happen before acquiring database transaction locks. A slow facilitator must not hold a credential row and a pool connection.
- HTTP admission is bounded to 32 active requests, 120 requests per socket IP per minute, 4096 tracked IPs, a 128 KiB API body limit, a 5-second body-read timeout, and a 180-second execution deadline. Forwarded-IP headers are not trusted. Upstream request, body, and execution limits remain active.
- Unexpected Host values, cross-origin requests, duplicate Origin headers, duplicate credential/idempotency/webhook headers, oversized signatures and forged webhooks are rejected. Sensitive API responses use `Cache-Control: no-store`.
- Dodo canonical payments must match the quoted cents exactly. Checkout metadata, product, session, payment ID and wallet must also agree before booking payment.
- Publisher-written names/descriptions are not interpolated into MCP tool instructions. Scout metadata and repository outputs remain untrusted data and must be treated that way by clients.

## Reproduce

```sh
devenv shell -- env HIVE_TEST_DATABASE_URL=postgres://hive:hive-local-only@127.0.0.1:5432/hive \
  cargo test --locked --workspace --all-targets -- --include-ignored
devenv shell -- cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
devenv shell -- cargo deny --locked check
```

Integration tests use isolated temporary schemas, real PostgreSQL constraints/transactions, and Axum's HTTP request pipeline. They cover cross-credential access, scope/revocation enforcement, signature tampering/replay, SQL search injection, mass assignment, hostile origins, duplicate credentials, oversized bodies, forged webhooks, and budget races. Unit tests exercise SSRF payloads and bounded admission windows/maps.

## Remaining limits

- The limiter is process-local. Public deployment still needs appropriate ingress limits, TLS, and a correctly configured exact frontend origin. Distributed denial-of-service protection is not established by these tests.
- Public GitHub requests now use unauthenticated API quotas; no private-repository support is offered.
- Local/public-chain settlement, reorg/finality behavior, completed Dodo checkout, refund/payout operations, and crash recovery still need end-to-end verification. Do not interpret adapter/unit coverage as completed financial acceptance testing.
- No verified savings or live OpenCode benchmark exists yet.
