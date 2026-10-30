import { cookies } from "next/headers";
import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

import { hiveway } from "../../../../lib/hiveway";
import { readOAuthHandshake } from "../../../../lib/oauth";
import {
  appOrigin,
  cookieOptions,
  sessionCookie,
} from "../../../../lib/server/session";

export const GET = async (request: NextRequest) => {
  const jar = await cookies();
  const raw = jar.get("hive_oauth")?.value;
  jar.delete("hive_oauth");
  let next = "/dashboard";
  try {
    const code = request.nextUrl.searchParams.get("code");
    const pending = readOAuthHandshake(
      raw,
      request.nextUrl.searchParams.get("state"),
      code
    );
    ({ next } = pending);
    const auth = await hiveway<{ token: string; expires_at: number }>(
      "/v1/auth/github",
      "",
      {
        body: JSON.stringify({ code, verifier: pending.verifier }),
        method: "POST",
      }
    );
    const response = NextResponse.redirect(`${appOrigin()}${next}`);
    response.cookies.set(sessionCookie, auth.token, {
      ...cookieOptions,
      expires: new Date(auth.expires_at * 1000),
    });
    response.headers.set("Cache-Control", "no-store");
    return response;
  } catch {
    return NextResponse.redirect(
      `${appOrigin()}/sign-in?${new URLSearchParams({ error: "github", next })}`
    );
  }
};
