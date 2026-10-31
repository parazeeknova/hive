# Private MCP connections

Run external MCPs through your authenticated Hive endpoint. A connector runs beside the upstream server and pulls account-private jobs over HTTPS. Hive never accepts a command or upstream URL from a tool caller. Any stdio or Streamable HTTP MCP can use this bridge; there is no filesystem-specific gateway code.

Hive routing is free and requires no wallet. Upstream billing still applies. Connections are private to the account that registers them and are not public marketplace artifacts. Use a dedicated, revocable Hive API key with `capabilities:publish` and `capabilities:invoke` scopes. Store it outside every filesystem allowed directory.

## Free filesystem MCP

The official `@modelcontextprotocol/server-filesystem` package is included. Install dependencies with `bun install`, create an empty workspace directory, and save this local trusted configuration outside that workspace:

```json
{
  "name": "filesystem",
  "hive": "https://hiveway.itssingularity.com/mcp",
  "keyFile": "/run/secrets/hive-key",
  "transport": "stdio",
  "command": "bun",
  "args": [
    "/app/node_modules/@modelcontextprotocol/server-filesystem/dist/index.js",
    "/workspace"
  ]
}
```

Use actual absolute paths for your installation. Run `bun packages/connector/src/index.ts /path/to/config.json`. Save the printed `connectionId` in the config before restarting to reuse the connection. Remove old entries with Hive's `disconnect_mcp` tool. Reconnect your harness to refresh its tool list. Call the returned `external_<connection-id>_<tool>` names through Hive.

For a hosted private workspace, run one connector per account in a container with a persistent `/workspace` volume and that account's key mounted separately as a read-only secret. Do not mount the Docker socket, host home directory, source checkout, or other accounts' volumes. A hosted workspace contains remote files; accessing your computer requires running the connector there with explicit allowed directories. This package does not automatically provision hosted containers.

## Other MCP servers

For stdio, replace `command`, `args` and optional `env` in trusted local configuration. Only explicitly provided environment overrides are passed alongside the SDK's minimal inherited environment. For HTTP, use `"transport": "http"`, `"url": "https://your-server.example/mcp"` and optional `headers`. Upstream secrets remain with the connector; Hive's key is never forwarded. There is no OAuth enrollment UI; complete any upstream authorization separately.

## Operation and limits

Gateway replicas share PostgreSQL connection records and indexed queues. Workers claim jobs with `SKIP LOCKED` and an unpredictable lease; completion is single-use. A connection is offline after 60 seconds without polling. Limits: eight connections per account, 64 tools per connection, 16 pending calls per connection, 128 KiB arguments, 512 KiB results, and a 30-second gateway wait. A worker executes one call at a time, with a 25-second upstream timeout. Calls are never automatically retried. A timeout can have an unknown mutation outcome; inspect the upstream state before retrying. A connector stops on transport failures; supervise/restart it with its saved ID.

Upstream descriptions, schemas, and results are untrusted. Tool names are namespaced; callers must already have account invocation access. Upstream tool lists are snapshotted when registered; disconnect/re-register after changing tools. MCP resources, prompts, interactive sampling, and elicitation are not forwarded by this tools-only bridge. Invocation receipts and the dashboard activity feed currently cover catalog artifacts, not connector calls.

Build the optional worker image with `docker build -f docker/connector/Dockerfile -t hive-connector .`. Run it with a read-only root filesystem, `--cap-drop ALL --security-opt no-new-privileges --memory 256m --cpus 1 --pids-limit 64`, a writable persistent workspace volume owned by the image's `bun` user, and read-only mounts for `/run/secrets/connector.json` and `/run/secrets/hive-key`. Keep worker credentials/configuration out of the workspace. The gateway itself requires migration 16 before connecting.
