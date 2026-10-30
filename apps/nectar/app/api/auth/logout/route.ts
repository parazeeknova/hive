import { cookies } from "next/headers";
import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

import { hiveway, HivewayError } from "../../../../lib/hiveway";
import { appOrigin, sessionCookie } from "../../../../lib/server/session";

export const POST = async (request: NextRequest) => {
  if (request.headers.get("origin") !== appOrigin()) {
    return new NextResponse(null, { status: 403 });
  }
  const jar = await cookies();
  const token = jar.get(sessionCookie)?.value;
  if (token) {
    try {
      await hiveway("/v1/auth/logout", token, { body: "{}", method: "POST" });
    } catch (error) {
      if (!(error instanceof HivewayError && error.status === 401)) {
        return NextResponse.json(
          { error: "Unable to revoke session. Please retry." },
          { status: 503 }
        );
      }
    }
  }
  jar.delete(sessionCookie);
  return NextResponse.redirect(`${appOrigin()}/login`, 303);
};
