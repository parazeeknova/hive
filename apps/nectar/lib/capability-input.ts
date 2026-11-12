import type { RegistryCapability } from "./hiveway";

export const connectedCapability = (tool: RegistryCapability) =>
  tool.runtime === "connected_mcp" || Array.isArray(tool.definition.mcp_tools);

export const usesJsonInput = (tool: RegistryCapability) =>
  tool.runtime === "wasm" ||
  tool.runtime === "repository_scaffold" ||
  connectedCapability(tool);

export const capabilityInput = (tool: RegistryCapability): unknown => {
  const binding = {
    arguments: {},
    connection_id: "00000000-0000-0000-0000-000000000000",
  };
  if (tool.runtime === "connected_mcp") {
    return {
      ...binding,
      ...(tool.definition.provider ? { tool: tool.definition.tool_name } : {}),
    };
  }
  if (Array.isArray(tool.definition.mcp_tools)) {
    return { steps: tool.definition.mcp_tools.map(() => binding) };
  }
  if (tool.runtime === "wasm") {
    return tool.definition.sample_input ?? {};
  }
  return {
    ...(tool.runtime === "repository_scaffold"
      ? { project_name: "my-project" }
      : {}),
    repository_url: "https://github.com/octocat/Hello-World",
    revision: "7fd1a60b01f91b314f59955a4e4d4c63fbd0fa49",
  };
};
