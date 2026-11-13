// Probe a stdio MCP server: print tools with annotations and input schema keys.
import { Client } from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

const [command, ...args] = process.argv.slice(2);
if (!command) {
  throw new Error("usage: probe.ts <command> [args...]");
}
const transport = new StdioClientTransport({
  args,
  command,
  env: { HOME: process.env.HOME ?? "", PATH: process.env.PATH ?? "" },
  stderr: "ignore",
});
const client = new Client({ name: "probe", version: "1" });
await client.connect(transport);
const { tools } = await client.listTools();
for (const tool of tools) {
  const props = Object.keys(
    (tool.inputSchema as { properties?: object }).properties ?? {}
  );
  console.log(
    JSON.stringify({
      annotations: tool.annotations ?? null,
      name: tool.name,
      props,
      required: (tool.inputSchema as { required?: string[] }).required ?? [],
    })
  );
}
console.log(
  "instructions:",
  JSON.stringify(client.getInstructions?.() ?? null).slice(0, 300)
);
await client.close();
process.exit(0);
