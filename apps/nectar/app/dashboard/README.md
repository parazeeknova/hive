# Nectar workspace

The workspace shares the marketing site's Subjectivity/Manrope typography and rounded controls. `/docs` provides client-specific MCP setup for Claude Code, OpenCode, Codex, Pi, and Antigravity, with local brand assets and links to primary documentation.

## State and backend integration

A root TanStack Query provider shares cached server data across navigation. Session credentials stay in memory, are excluded from query keys, and are cleared on reload or disconnect. Replacing a session cancels pending requests and clears the query cache so another account cannot inherit prior account data. Queries accept cancellation signals, refresh on focus and invalidate after mutations.

The workspace connects wallet challenges, credential status, cursor-based discovery, activity/receipt reconciliation and creator sales. Catalog queries use `/v1/capabilities/page`, support server runtime filters, and load additional cursor pages. Account settings loads `mine=true&include_inactive=true` and supports pagination, so inactive listings remain manageable. Availability changes use the authenticated owner endpoint and invalidate cached queries.

Wasm tools accept JSON input and expose their real artifact schemas. Free quotes execute through the invocation endpoint and display the result. Paid quotes retain the explicit x402 handoff to the client's wallet; Nectar does not claim to submit that payment. Existing receipt polling preserves uncertain settlement states and backs off while hidden.

Code and prices are immutable. Connected accounts can pause or restore listings, but pricing controls are disabled and direct users to publish a new capability. The disconnected example catalog remains clearly separated from live server data.

## Local preferences and limits

Profile/avatar preferences remain browser-local because Hiveway has no profile API. The demo balance is explicitly local preview data; connected accounts do not interpret authorization budgets or earnings as spendable funds. The funding dialog does not fabricate checkout, token transfers or creator payouts.

Set `NEXT_PUBLIC_HIVEWAY_ORIGIN` for browser requests (development default `http://127.0.0.1:3001`) and configure Hiveway's exact Nectar origin for CORS. The server health route also accepts `HIVEWAY_ORIGIN`. Keep tokens private; Account settings can deliberately copy the connected credential for MCP setup. It expires with its session.

## Validation

Run `bun test scripts/tests`, `bun run check --ci`, Nectar type checks, and `bun --cwd apps/nectar next build`. CI includes the frontend contract tests and production build. Local browser acceptance exercises the five documentation clients, responsive layout and assets, authenticated catalog search, and a free hosted-artifact invocation against a running Hiveway instance.
