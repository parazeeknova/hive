import { expect, test } from "bun:test";

import { renderToStaticMarkup } from "react-dom/server";

import { ToolActivityRow } from "../../apps/nectar/app/dashboard/tool-activity-row";
import { activityEntries } from "../../apps/nectar/lib/activity";
import type { Activity, ToolActivity } from "../../apps/nectar/lib/hiveway";

const event: ToolActivity = {
  agent_name: "Dev agent",
  client_context: { harness: "test" },
  completed_at: 100,
  created_at: 100,
  error_code: "provider_unreachable",
  execution_ms: 42,
  id: "call-1",
  provider: "context7",
  run_id: "run-1",
  state: "failed",
  task_id: null,
  tool: "mcp_context7__query-docs",
};
test("activity includes provider errors and correlation without a payment receipt", () => {
  const html = renderToStaticMarkup(
    <ToolActivityRow item={event} status="Failed" />
  );
  for (const value of [
    event.tool,
    event.error_code,
    event.id,
    event.run_id,
    "42 ms",
  ]) {
    expect(html).toContain(value);
  }
  expect(html).not.toContain("USDC");
});
test("activity orders and deduplicates overlapping pagination results", () => {
  const activity: Activity = {
    budget: { limit: "0", reserved: "0", spent: "0" },
    invocations: [],
    tool_calls: [event, event, { ...event, created_at: 101, id: "call-2" }],
  };
  expect(activityEntries(activity).map((item) => item.id)).toEqual([
    "call-2",
    "call-1",
  ]);
  expect(activityEntries(null)).toEqual([]);
});
