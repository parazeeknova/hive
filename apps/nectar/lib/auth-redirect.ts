/** Only allow returning to the dashboard; never redirect OAuth to an external URL. */
export const dashboardReturnPath = (value: unknown): string => {
  if (typeof value !== "string" || value.length > 2048) {
    return "/dashboard";
  }
  try {
    const url = new URL(value, "https://hive.invalid");
    if (
      !value.startsWith("/") ||
      url.origin !== "https://hive.invalid" ||
      url.pathname !== "/dashboard"
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
