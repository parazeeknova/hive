import { cookies } from "next/headers";

import { hiveway } from "../hiveway";
import type { Credential } from "../hiveway";

export const sessionCookie = "hive_session";
export const cookieOptions = {
  httpOnly: true,
  path: "/",
  sameSite: "lax" as const,
  secure: process.env.NODE_ENV === "production",
};
export const session = async () => {
  const jar = await cookies();
  const token = jar.get(sessionCookie)?.value;
  if (!token) {
    return null;
  }
  try {
    return await hiveway<Credential>("/v1/credentials/me", token);
  } catch {
    return null;
  }
};
export const appOrigin = () => {
  const origin = process.env.HIVE_AUTH_ORIGIN ?? "http://127.0.0.1:3000";
  return new URL(origin).origin;
};
