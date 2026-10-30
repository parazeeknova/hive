/** Produce editable examples from published schemas; runtime validation remains authoritative. */
const arrayLength = (value: unknown) =>
  typeof value === "number" ? Math.min(16, Math.max(0, value)) : 1;

export const sampleInput = (schema: unknown, depth = 0): unknown => {
  if (!schema || typeof schema !== "object" || depth > 8) {
    return null;
  }
  const value = schema as Record<string, unknown>;
  if ("default" in value) {
    return value.default;
  }
  if ("const" in value) {
    return value.const;
  }
  if (Array.isArray(value.examples) && value.examples.length > 0) {
    return value.examples[0];
  }
  if (Array.isArray(value.enum) && value.enum.length > 0) {
    return value.enum[0];
  }
  if (value.type === "object" || value.properties) {
    const { properties } = value;
    if (!properties || typeof properties !== "object") {
      return {};
    }
    return Object.fromEntries(
      Object.entries(properties)
        .slice(0, 64)
        .map(([key, child]) => [key, sampleInput(child, depth + 1)])
    );
  }
  if (value.type === "array") {
    return Array.from(
      {
        length: arrayLength(value.minItems),
      },
      () => sampleInput(value.items, depth + 1)
    );
  }
  if (value.type === "boolean") {
    return false;
  }
  if (["number", "integer"].includes(String(value.type))) {
    return typeof value.minimum === "number" ? value.minimum : 0;
  }
  if (value.type === "string") {
    return "sample";
  }
  return null;
};
