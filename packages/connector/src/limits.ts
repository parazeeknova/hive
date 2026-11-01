import { Buffer } from "node:buffer";

export const withinResultLimit = (value: unknown) =>
  Buffer.byteLength(JSON.stringify(value) ?? "", "utf-8") <= 524_288;
