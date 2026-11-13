import type { ReactNode } from "react";

import type { ToolActivity, Activity } from "../../lib/hiveway";

const executionTime = (item: ToolActivity) => {
  if (item.completed_at === null) {
    return "In progress";
  }
  return item.execution_ms === null ? "—" : `${item.execution_ms} ms`;
};

export const ToolActivityRow = ({
  item,
  status,
}: {
  item: ToolActivity;
  status: ReactNode;
}) => (
  <details className="border-t border-[var(--ws-line)] py-3 text-[11px] text-[#686f60]">
    <summary className="flex cursor-pointer items-center gap-4">
      <span className="min-w-0 flex-1">
        <strong className="block break-all">{item.tool}</strong>
        <small className="mt-1 block">
          {new Date(item.created_at * 1000).toLocaleString()} ·{" "}
          {item.provider ?? "Hive"} · {item.agent_name}
        </small>
      </span>
      {status}
      <span>{executionTime(item)}</span>
    </summary>
    <dl className="mt-3 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 font-mono text-[10px] break-all">
      <dt>Call</dt>
      <dd>{item.id}</dd>
      {item.error_code && (
        <>
          <dt>Error</dt>
          <dd>{item.error_code}</dd>
        </>
      )}
      {item.run_id && (
        <>
          <dt>Workflow</dt>
          <dd>{item.run_id}</dd>
        </>
      )}
      {item.task_id && (
        <>
          <dt>Task</dt>
          <dd>{item.task_id}</dd>
        </>
      )}
      {item.client_context?.harness && (
        <>
          <dt>Client</dt>
          <dd>{item.client_context.harness}</dd>
        </>
      )}
    </dl>
  </details>
);

export const ToolActivityCounts = ({
  counts,
}: {
  counts: Activity["tool_counts"];
}) =>
  counts ? (
    <p className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
      {counts.total} MCP calls · {counts.succeeded} succeeded · {counts.failed}{" "}
      failed
    </p>
  ) : null;
