# Local Hiveway tooling

Development-only dependencies for upcoming EVM payment fixtures, the official x402 facilitator, and MCP interoperability tests. The production Rust backend does not require this package. Install from the repository root with `bun install`; the root lockfile owns this workspace.

The end-to-end runner and Docker payment services are pending. Keeping these dependencies separate prevents test-chain tooling from entering Nectar or Hiveway's runtime.

Hiveway operational values live in `apps/hiveway/config.toml`. Set `HIVE_CONFIG_FILE` to load a different configuration file; environment URL overrides and provider credentials are read at startup. Invalid or inconsistent resource limits prevent startup. Test wallet addresses and signatures stay inside fixtures.
