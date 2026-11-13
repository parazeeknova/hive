// Call tools on a stdio MCP server in one session: call.ts '<json calls>' <command> [args...]
import { Client } from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

const [calls, command, ...args] = process.argv.slice(2);
if (!calls || !command) {
  throw new Error("usage: call.ts '<[{name,arguments}]>' <command> [args...]");
}
const transport = new StdioClientTransport({
  args,
  command,
  env: { HOME: process.env.HOME ?? "", PATH: process.env.PATH ?? "" },
  stderr: "inherit",
});
const client = new Client({ name: "probe", version: "1" });
await client.connect(transport);
for (const call of JSON.parse(calls) as {
  name: string;
  arguments: Record<string, unknown>;
}[]) {
  try {
    // oxlint-disable-next-line no-await-in-loop -- Calls run in order on one session.
    const result = await client.callTool(call);
    console.log(
      call.name,
      result.isError ? "ERROR" : "ok",
      JSON.stringify(result.content).slice(0, 400)
    );
  } catch (error) {
    console.log(call.name, "THROW", String(error).slice(0, 300));
  }
}
await client.close();
process.exit(0);
