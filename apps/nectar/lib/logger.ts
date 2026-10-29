import { context, logs, trace } from "@opentelemetry/api";
import pino from "pino";

// A Pino stream receives already serialized and redacted records. Mirror those
// records to OTLP while Pino retains JSON output in the local terminal.
const otelStream = {
  write(line: string) {
    const record: Record<string, unknown> = JSON.parse(line);
    logs.getLogger("nectar").emit({
      body: line.trim(),
      severityNumber:
        typeof record.level === "number" ? (record.level / 10 - 1) * 4 + 1 : 9,
      severityText:
        typeof record.level === "number"
          ? (pino.levels.labels[record.level] ?? "info")
          : "info",
    });
  },
};

export const logger = pino(
  {
    base: { service: "nectar" },
    level: process.env.LOG_LEVEL ?? "info",
    mixin() {
      const span = trace.getSpan(context.active())?.spanContext();
      return span ? { span_id: span.spanId, trace_id: span.traceId } : {};
    },
    redact: [
      "password",
      "apiKey",
      "authorization",
      "headers.authorization",
      "headers.cookie",
      "req.headers.authorization",
      "req.headers.cookie",
    ],
  },
  pino.multistream([{ stream: process.stdout }, { stream: otelStream }])
);
