import { cookies } from "next/headers";
import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

import { apiOrigin } from "../../../../lib/hiveway";
import { appOrigin, sessionCookie } from "../../../../lib/server/session";

const proxy = async (
  request: NextRequest,
  context: { params: Promise<{ path: string[] }> }
) => {
  const jar = await cookies();
  const token = jar.get(sessionCookie)?.value;
  if (!token) {
    return NextResponse.json({ error: "Sign in required" }, { status: 401 });
  }
  if (
    request.method !== "GET" &&
    request.headers.get("origin") !== appOrigin()
  ) {
    return new NextResponse(null, { status: 403 });
  }
  const { path } = await context.params;
  if (
    path[0] !== "v1" ||
    path.some((part) => !/^[a-zA-Z0-9_-]+$/u.test(part))
  ) {
    return new NextResponse(null, { status: 404 });
  }
  if (path[1] === "auth") {
    return new NextResponse(null, { status: 403 });
  }
  const headers = new Headers({ Authorization: `Bearer ${token}` });
  for (const name of ["content-type", "idempotency-key", "payment-signature"]) {
    const value = request.headers.get(name);
    if (value) {
      headers.set(name, value);
    }
  }
  const response = await fetch(
    `${apiOrigin}/${path.join("/")}${request.nextUrl.search}`,
    {
      cache: "no-store",
      headers,
      method: request.method,
      signal: AbortSignal.timeout(180_000),
      ...(request.method === "GET" ? {} : { body: await request.text() }),
    }
  );
  return new NextResponse(response.body, {
    headers: {
      "Cache-Control": "no-store",
      "Content-Type":
        response.headers.get("content-type") ?? "application/json",
    },
    status: response.status,
  });
};
export { proxy as GET, proxy as POST };
