import { cookies } from "next/headers";
import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

import { hiveway } from "../../../../lib/hiveway";
import { authorizeRequest, consentPath } from "../../../../lib/oauth-consent";
import { appOrigin, sessionCookie } from "../../../../lib/server/session";

/** Record the signed-in user's consent decision and send them back to the MCP client. */
export const POST = async (request: NextRequest) => {
  // The consent form is same-origin; anything else is a forged decision.
  if (request.headers.get("origin") !== appOrigin()) {
    return new NextResponse(null, { status: 403 });
  }
  const jar = await cookies();
  const token = jar.get(sessionCookie)?.value;
  const form = await request.formData();
  const params = authorizeRequest(form);
  if (!token) {
    return NextResponse.redirect(
      `${appOrigin()}/sign-in?${new URLSearchParams({ next: consentPath(params) })}`,
      303
    );
  }
  const decision = form.get("decision") === "approve" ? "approve" : "deny";
  try {
    const result = await hiveway<{ redirect: string }>(
      "/v1/oauth/authorize",
      token,
      {
        body: JSON.stringify({ ...params, decision }),
        method: "POST",
      }
    );
    const response = NextResponse.redirect(result.redirect, 303);
    response.headers.set("Cache-Control", "no-store");
    return response;
  } catch {
    return NextResponse.redirect(`${appOrigin()}${consentPath(params)}`, 303);
  }
};
