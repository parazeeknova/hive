import { OTLPLogExporter } from "@opentelemetry/exporter-logs-otlp-http";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-http";
import { UndiciInstrumentation } from "@opentelemetry/instrumentation-undici";
import { resourceFromAttributes } from "@opentelemetry/resources";
import { BatchLogRecordProcessor } from "@opentelemetry/sdk-logs";
import { NodeSDK } from "@opentelemetry/sdk-node";

let sdk: NodeSDK | undefined;

export const startTelemetry = () => {
  if (sdk) {
    return;
  }
  sdk = new NodeSDK({
    instrumentations: [new UndiciInstrumentation()],
    logRecordProcessors: [
      new BatchLogRecordProcessor({ exporter: new OTLPLogExporter() }),
    ],
    resource: resourceFromAttributes({
      "service.name": "nectar",
      "service.namespace": "hive",
      "service.version": "0.0.0",
    }),
    traceExporter: new OTLPTraceExporter(),
  });
  sdk.start();
  const shutdown = async () => {
    try {
      await sdk?.shutdown();
    } catch (error) {
      console.error("Nectar telemetry shutdown failed", error);
    }
  };
  process.once("SIGTERM", shutdown);
  process.once("SIGINT", shutdown);
};
