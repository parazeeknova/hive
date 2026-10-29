export const register = async () => {
  if (process.env.NEXT_RUNTIME === "nodejs") {
    const { startTelemetry } = await import("./lib/telemetry");
    startTelemetry();
  }
};

export const onRequestError = async (error: unknown) => {
  if (process.env.NEXT_RUNTIME === "nodejs") {
    const { logger } = await import("./lib/logger");
    logger.error({ err: error }, "Nectar request failed");
  }
};
