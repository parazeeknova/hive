import type { Activity, Invocation, ToolActivity } from "./hiveway";

/** Merge the paginated feed while allowing a refreshed page to overlap an older one. */
export const activityEntries = (
  activity: Activity | null
): (Invocation | ToolActivity)[] => {
  const entries = [
    ...(activity?.invocations ?? []),
    ...(activity?.tool_calls ?? []),
  ];
  return [
    ...new Map(entries.map((entry) => [entry.id, entry])).values(),
  ].toSorted((a, b) => b.created_at - a.created_at || b.id.localeCompare(a.id));
};
