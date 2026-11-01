import type { Client } from "@modelcontextprotocol/client";

export class FatalHiveError extends Error {
  override name = "FatalHiveError";
}

export const reconnect = async (
  client: Pick<Client, "connect">,
  transport: Parameters<Client["connect"]>[0]
) => {
  try {
    await client.connect(transport);
  } catch {
    throw new FatalHiveError("Filesystem MCP reconnection failed");
  }
};
