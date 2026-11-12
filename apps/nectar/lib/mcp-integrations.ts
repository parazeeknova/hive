import type { RegistryCapability } from "./hiveway";

/** Group only backend-curated integration IDs; a user's chosen name grants no badge or grouping. */
export const groupConnectedCapabilities = (catalog: RegistryCapability[]) => {
  const groups = new Map<string, RegistryCapability[]>();
  const result: RegistryCapability[] = [];
  for (const capability of catalog) {
    const integration = capability.metadata?.integration;
    if (capability.runtime !== "connected_mcp" || !integration) {
      result.push(capability);
      continue;
    }
    const group = groups.get(integration.slug) ?? [];
    group.push(capability);
    groups.set(integration.slug, group);
  }
  for (const members of groups.values()) {
    const integration = members[0]?.metadata?.integration;
    if (!integration) {
      continue;
    }
    const primary =
      members.find((capability) => capability.id === integration.id) ??
      members[0];
    if (!primary) {
      continue;
    }
    result.push({
      ...primary,
      description: integration.description,
      members,
      name: integration.name,
    });
  }
  return result;
};
