import { randomBytes, createHash } from "node:crypto";

import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

import { dashboardReturnPath } from "../../../../lib/auth-redirect";
import { appOrigin, cookieOptions } from "../../../../lib/server/session";

export const GET = (request: NextRequest) => {
  const clientId = process.env.GITHUB_CLIENT_ID;
  if (!clientId) {
    return NextResponse.json(
      { error: "GitHub OAuth is not configured." },
      { status: 503 }
    );
  }
  const state = randomBytes(32).toString("base64url");
  const verifier = randomBytes(32).toString("base64url");
  const url = new URL("https://github.com/login/oauth/authorize");
  url.search = new URLSearchParams({
    client_id: clientId,
    code_challenge: createHash("sha256").update(verifier).digest("base64url"),
    code_challenge_method: "S256",
    redirect_uri: `${appOrigin()}/api/auth/callback`,
    state,
  }).toString();
  const response = NextResponse.redirect(url);
  response.cookies.set(
    "hive_oauth",
    JSON.stringify({
      next: dashboardReturnPath(request.nextUrl.searchParams.get("next")),
      state,
      verifier,
    }),
    {
      ...cookieOptions,
      maxAge: 600,
    }
  );
  response.headers.set("Cache-Control", "no-store");
  return response;
};
