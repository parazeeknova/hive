/** Parameters an MCP client sends to `/oauth/authorize`, forwarded verbatim to Hiveway. */
export const AUTHORIZE_FIELDS = [
  "client_id",
  "redirect_uri",
  "response_type",
  "code_challenge",
  "code_challenge_method",
  "scope",
  "state",
  "resource",
] as const;
export type AuthorizeField = (typeof AUTHORIZE_FIELDS)[number];
export type AuthorizeRequest = Partial<Record<AuthorizeField, string>>;

export interface Consent {
  client: {
    client_id: string;
    name: string;
    uri: string | null;
    verified_by: "client_metadata_document" | "dynamic_registration";
    redirect_host: string;
  };
  scopes: { scope: string; description: string }[];
  account: string;
  redirect?: string;
}

/** First value of each known field; unknown parameters are dropped. */
export const authorizeRequest = (
  source: Record<string, string | string[] | undefined> | FormData
): AuthorizeRequest => {
  const request: AuthorizeRequest = {};
  for (const field of AUTHORIZE_FIELDS) {
    const raw = source instanceof FormData ? source.get(field) : source[field];
    const value = Array.isArray(raw) ? raw[0] : raw;
    if (typeof value === "string" && value.length > 0 && value.length <= 2048) {
      request[field] = value;
    }
  }
  return request;
};

/** The consent URL to come back to after signing in. */
export const consentPath = (request: AuthorizeRequest) =>
  `/oauth/authorize?${new URLSearchParams(
    Object.entries(request).filter((entry): entry is [string, string] =>
      Boolean(entry[1])
    )
  )}`;
