/** In-app pages sign-in may return to: the dashboard and MCP OAuth consent. */
const RETURN_PATHS = new Set(["/dashboard", "/oauth/authorize"]);
/** Only allow returning to known in-app pages; never redirect sign-in to an external URL. */
export const dashboardReturnPath = (value: unknown): string => {
  if (typeof value !== "string" || value.length > 2048) {
    return "/dashboard";
  }
  try {
    const url = new URL(value, "https://hive.invalid");
    if (
      !value.startsWith("/") ||
      value.startsWith("//") ||
      url.origin !== "https://hive.invalid" ||
      !RETURN_PATHS.has(url.pathname)
    ) {
      return "/dashboard";
    }
    return `${url.pathname}${url.search}`;
  } catch {
    return "/dashboard";
  }
};
export const dashboardDestination = (
  params: Record<string, string | string[] | undefined>
) => {
  const query = new URLSearchParams();
  for (const key of ["q", "view", "tab", "tool"]) {
    const value = params[key];
    const first = Array.isArray(value) ? value[0] : value;
    if (first) {
      query.set(key, first);
    }
  }
  return dashboardReturnPath(`/dashboard${query.size ? `?${query}` : ""}`);
};
