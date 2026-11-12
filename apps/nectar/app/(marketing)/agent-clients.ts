export const clients = [
  {
    command:
      // oxlint-disable-next-line no-template-curly-in-string -- Claude resolves this stored environment reference.
      'claude mcp add-json --scope project hive \'{"type":"http","url":"https://hiveway.itssingularity.com/mcp","headers":{"Authorization":"Bearer ${HIVE_API_KEY}"}}\'',
    id: "claude",
    name: "Claude Code",
  },
  {
    command:
      "codex mcp add hive --url https://hiveway.itssingularity.com/mcp --bearer-token-env-var HIVE_API_KEY",
    id: "codex",
    name: "Codex",
  },
  {
    command:
      "opencode mcp add hive --url https://hiveway.itssingularity.com/mcp --header 'Authorization=Bearer {env:HIVE_API_KEY}' --global",
    id: "opencode",
    name: "OpenCode",
  },
] as const;
