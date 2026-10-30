const demoNames: Record<string, string> = {
  "free mcp echo": "Agent echo",
  "hosted echo": "JSON echo",
  "local repository stack": "Repository stack report",
};
// Old demo runs appended IDs to the title. Published capability IDs remain unchanged.
export const capabilityDisplayName = (name: string) => {
  const match =
    /^(?<demo>Local repository stack|Hosted echo|Free MCP echo) [\da-f]{8}-[\da-f]{4}-[\da-f]{4}-[\da-f]{4}-[\da-f]{12}$/iu.exec(
      name
    );
  const prefix = match?.groups?.demo?.toLowerCase();
  return prefix ? (demoNames[prefix] ?? name) : name;
};
