# Contributing to Hive

Hive is built around a hosted MCP endpoint, a Rust gateway/workflow engine, a native Rust local worker and a web dashboard. Contributions should make those parts work together and preserve account boundaries.

## Get a checkout running

Follow the [project README](.github/README.md#develop-and-self-host). Use the Rust version in `rust-toolchain.toml` and Bun version in `package.json`. Keep credentials in local files or secret storage, outside approved tool folders. Use disposable databases for integration tests.

## Add an MCP integration

1. Link the original project, name its maintainers, and check its license and runtime prerequisites. Update the credits table in the README.
2. Decide where it runs: caller-private local worker, a configured upstream HTTP provider, or an isolated shared service. A shared service must not inherit access to user files or credentials.
3. Add its entry to [integrations.json](apps/hiveway/integrations.json). Pin the package and supported Python runtime, allowlist actual tools, and provide accurate side-effect annotations. Path/URL arguments need appropriate validation.
4. Test the real server's initialize, tool-list and call behavior through the bridge. Check failure, cancellation, result sizes, folder boundaries and whether the tool is safe to cache. Test Windows, macOS and Linux for local integrations; do not assume a Unix launch command works on Windows.
5. Connect it to an existing capability or add a useful recipe in [workflows.json](apps/hiveway/workflows.json). Workflow steps use the caller's credentials and connections.
6. Document setup and limitations. Registering an integration does not provision a shared upstream automatically.

The gateway syncs compiled-in integration and official workflow changes at startup. Running helpers refresh their catalog and tool snapshots. Package identity changes require fresh local approval; pinned version changes can be applied between jobs.

## Validate a change

Run the checks in the [README](.github/README.md#verify-your-changes). For live upstreams, set `HIVE_TEST_UPSTREAMS=1` and run `cargo test -p hive-local -- --include-ignored`; use `$env:HIVE_TEST_UPSTREAMS="1"` in PowerShell. The shared sandbox test additionally needs the supplied proxy and is intended for Linux-container infrastructure.

The CI platform matrix runs native helper tests, the five bundled local upstreams, and gateway unit tests on Windows, macOS and Linux. PostgreSQL/security integration tests and shared-service infrastructure run in the Linux environment. Report the actual platforms and checks you exercised in your pull request.

## Keep changes reviewable

Explain the problem, resulting behavior, validation and remaining limitations. Preserve existing user work. Avoid introducing dependencies for features already supplied by the gateway or bridge. Treat upstream tool descriptions, documents and publisher text as data, rather than instructions that override the caller's permissions.

## Documentation artwork

README diagrams use the existing Hive mark and Manrope font. Edit [generate-readme-assets.ts](scripts/generate-readme-assets.ts), then run `bun scripts/generate-readme-assets.ts` with `rsvg-convert` installed. Commit the editable SVGs and exported PNGs together. Preserve accessible alt text and upstream attribution.
