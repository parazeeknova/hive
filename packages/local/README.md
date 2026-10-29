# Local Hiveway tooling

Development-only dependencies for upcoming EVM payment fixtures, the official x402 facilitator, and MCP interoperability tests. The production Rust backend does not require this package. Install from the repository root with `bun install`; the root lockfile owns this workspace.

The end-to-end runner and Docker payment services are pending. Keeping these dependencies separate prevents test-chain tooling from entering Nectar or Hiveway's runtime.
