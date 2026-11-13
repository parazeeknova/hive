import { Buffer } from "node:buffer";

/** Hive's default connector result ceiling; deployments publish their own at `/api/config`. */
export const DEFAULT_RESULT_BYTES = 524_288;

export const withinResultLimit = (
  value: unknown,
  limit: number = DEFAULT_RESULT_BYTES
) => Buffer.byteLength(JSON.stringify(value) ?? "", "utf-8") <= limit;
