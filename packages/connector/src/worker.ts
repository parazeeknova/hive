import type { HiveRpc } from "./hive-rpc";
import { withinResultLimit } from "./limits";
import { FatalHiveError } from "./session";
import { failure } from "./upstream";
import type { ClaimedJob, Outcome } from "./upstream";

/** One connection a worker serves. */
export interface Service {
  name: string;
  id?: string;
  run: (job: ClaimedJob) => Promise<Outcome>;
}

const ignore = () => null;

/** Report exactly one outcome. Never execute a claimed job twice. */
export const completeJob = async (
  rpc: HiveRpc,
  connectionId: string,
  job: ClaimedJob,
  outcome: Outcome,
  resultLimit: number
) => {
  let report: Outcome = outcome;
  if ("result" in outcome && !withinResultLimit(outcome.result, resultLimit)) {
    report = { result: failure as unknown as Record<string, unknown> };
  }
  await rpc.operation("complete_mcp", {
    connection_id: connectionId,
    job_id: job.id,
    lease: job.lease,
    ...report,
  });
};

export interface ServeOptions {
  rpc: HiveRpc;
  services: Service[];
  /** Long-poll duration; zero falls back to interval polling for older Hive servers. */
  waitSeconds: number;
  pollIntervalMs: number;
  resultLimit: number;
  isRunning: () => boolean;
}

/**
 * Claim and execute jobs until stopped. One long-poll covers every idle
 * service, so adding providers does not multiply request volume. Each service
 * runs one job at a time; different services run concurrently.
 */
export const serve = async (options: ServeOptions) => {
  const { rpc, services, waitSeconds, pollIntervalMs, resultLimit } = options;
  const busy = new Set<string>();
  const active = new Set<Promise<void>>();
  const stop: { fatal: FatalHiveError | null } = { fatal: null };
  let delay = 1000;
  const dispatch = (service: Service, job: ClaimedJob) => {
    const id = service.id as string;
    busy.add(id);
    const execute = async () => {
      let outcome: Outcome;
      try {
        outcome = await service.run(job);
      } catch (error) {
        if (error instanceof FatalHiveError) {
          stop.fatal = error;
          return;
        }
        outcome = { result: failure as unknown as Record<string, unknown> };
      }
      try {
        await completeJob(rpc, id, job, outcome, resultLimit);
      } catch (error) {
        if (error instanceof FatalHiveError) {
          stop.fatal = error;
          return;
        }
        console.error(
          "Hive connector: completion was not acknowledged; the claimed call will not be repeated."
        );
      }
    };
    const task = execute().finally(() => {
      busy.delete(id);
      active.delete(task);
    });
    active.add(task);
  };
  while (options.isRunning()) {
    if (stop.fatal) {
      throw stop.fatal;
    }
    const idle = services.filter((s) => s.id && !busy.has(s.id));
    if (idle.length === 0) {
      // oxlint-disable-next-line no-await-in-loop -- Wait for a running job before claiming more.
      await Promise.race([...active, Bun.sleep(pollIntervalMs)]).catch(ignore);
      continue;
    }
    const started = Date.now();
    try {
      const args =
        idle.length === 1
          ? { connection_id: idle[0]?.id, wait_seconds: waitSeconds }
          : {
              connection_ids: idle.map((s) => s.id),
              wait_seconds: waitSeconds,
            };
      // oxlint-disable-next-line no-await-in-loop -- Sequential long-polls are the worker's heartbeat.
      const polled = await rpc.operation(
        idle.length === 1 ? "poll_mcp" : "poll_mcp_batch",
        args,
        (waitSeconds + 15) * 1000
      );
      delay = 1000;
      const job = polled.job as ClaimedJob | null | undefined;
      if (job && typeof job === "object") {
        const service =
          idle.length === 1
            ? idle[0]
            : idle.find((s) => s.id === job.connection_id);
        if (service) {
          dispatch(service, job);
        }
        continue;
      }
      // Servers without long-poll answer at once; pace those polls.
      if (Date.now() - started < 1000) {
        // oxlint-disable-next-line no-await-in-loop -- Interval polling for servers without long-poll.
        await Bun.sleep(pollIntervalMs);
      }
    } catch (error) {
      if (error instanceof FatalHiveError) {
        throw error;
      }
      // oxlint-disable-next-line no-await-in-loop -- Back off after transport failures.
      await Bun.sleep(delay);
      delay = Math.min(10_000, delay * 2);
    }
  }
  await Promise.allSettled(active);
};
