import { expect, test } from "bun:test";

import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

import {
  FatalHiveError,
  reconnect,
} from "../../packages/connector/src/session";

test("failed replacement connections require a worker restart", async () => {
  const transport = new StdioClientTransport({
    command: "unused-test-command",
  });
  const client = {
    connect: () => Promise.reject(new Error("upstream failed")),
  };
  await expect(reconnect(client, transport)).rejects.toBeInstanceOf(
    FatalHiveError
  );
});

test("replacement connection success is preserved", async () => {
  const transport = new StdioClientTransport({
    command: "unused-test-command",
  });
  let connected = false;
  await reconnect(
    {
      connect: () => {
        connected = true;
        return Promise.resolve();
      },
    },
    transport
  );
  expect(connected).toBe(true);
});
