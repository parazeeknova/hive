import { dashboardReturnPath } from "./auth-redirect";

/** Validate the browser-bound, short-lived OAuth handshake before exchanging a code. */
export const readOAuthHandshake = (
  raw: string | undefined,
  state: string | null,
  code: string | null
): { state: string; verifier: string; next: string } => {
  const pending: unknown = JSON.parse(raw ?? "null");
  if (
    !code ||
    !state ||
    !pending ||
    typeof pending !== "object" ||
    !("state" in pending) ||
    pending.state !== state ||
    !("verifier" in pending) ||
    typeof pending.verifier !== "string" ||
    !/^[A-Za-z0-9_-]{43,128}$/u.test(pending.verifier)
  ) {
    throw new Error("Invalid OAuth state");
  }
  return {
    next: dashboardReturnPath("next" in pending ? pending.next : undefined),
    state,
    verifier: pending.verifier,
  };
};
