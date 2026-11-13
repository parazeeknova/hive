import { cookies } from "next/headers";
import Link from "next/link";
import { redirect } from "next/navigation";

import { Brand } from "../../../components/brand";
import { errorMessage, hiveway } from "../../../lib/hiveway";
import {
  AUTHORIZE_FIELDS,
  authorizeRequest,
  consentPath,
} from "../../../lib/oauth-consent";
import type { Consent } from "../../../lib/oauth-consent";
import { privateMetadata } from "../../../lib/seo";
import { sessionCookie } from "../../../lib/server/session";

export const metadata = { ...privateMetadata, title: "Connect an app — Hive" };

const button =
  "inline-flex min-h-[42px] items-center justify-center rounded-full px-[18px] text-[15px] leading-none font-medium transition-colors duration-300 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-[#24251f]";

const Shell = ({ children }: { children: React.ReactNode }) => (
  <div className="font-body flex min-h-dvh flex-col items-center gap-8 bg-[#fffefb] px-6 py-10 text-[17px] leading-[1.55] text-[#24251f]">
    <Brand markClassName="size-8" wordmarkClassName="text-3xl" />
    <main
      id="main"
      className="w-full max-w-[520px] rounded-[22px] border border-[#dedfd7] bg-white p-8"
    >
      {children}
    </main>
  </div>
);

/**
 * MCP OAuth consent. Remote MCP clients send the user here; the signed-in
 * GitHub session approves or denies a scoped, revocable grant. Hiveway
 * validates the client and redirect URI before anything is shown.
 */
const AuthorizePage = async ({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) => {
  const request = authorizeRequest(await searchParams);
  const jar = await cookies();
  const token = jar.get(sessionCookie)?.value;
  if (!token) {
    redirect(`/sign-in?${new URLSearchParams({ next: consentPath(request) })}`);
  }
  let consent: Consent;
  try {
    consent = await hiveway<Consent>("/v1/oauth/authorize", token, {
      body: JSON.stringify({ ...request, decision: "inspect" }),
      method: "POST",
    });
  } catch (error) {
    return (
      <Shell>
        <h1 className="font-display text-3xl">This app cannot connect</h1>
        <p className="my-4 text-[#727268]" role="alert">
          The request from this MCP client is invalid: {errorMessage(error)}.
          Nothing was shared.
        </p>
        <Link className="underline" href="/dashboard">
          Go to your dashboard
        </Link>
      </Shell>
    );
  }
  if (consent.redirect) {
    // Hiveway rejected the request after validating its redirect URI.
    redirect(consent.redirect);
  }
  const verified =
    consent.client.verified_by === "client_metadata_document"
      ? `Identity published at ${consent.client.client_id}`
      : "Self-registered app; its name is not verified";
  return (
    <Shell>
      <h1 className="font-display text-3xl">
        Connect {consent.client.name} to Hive?
      </h1>
      <p className="mt-2 text-[15px] text-[#727268]">{verified}</p>
      <p className="mt-4">
        Signed in as <strong>{consent.account}</strong>. After you approve, you
        return to <strong>{consent.client.redirect_host}</strong>.
      </p>
      <h2 className="mt-6 text-[15px] font-semibold tracking-wide text-[#727268] uppercase">
        This app will be able to
      </h2>
      <ul className="mt-2 list-disc pl-6">
        {consent.scopes.map((item) => (
          <li key={item.scope}>{item.description}</li>
        ))}
      </ul>
      <p className="mt-4 text-[15px] text-[#727268]">
        It cannot spend money or create API keys. You can disconnect it at any
        time from your dashboard.
      </p>
      <form
        action="/api/oauth/decision"
        method="post"
        className="mt-6 flex gap-3"
      >
        {AUTHORIZE_FIELDS.map((field) =>
          request[field] ? (
            <input
              key={field}
              type="hidden"
              name={field}
              value={request[field]}
            />
          ) : null
        )}
        <button
          type="submit"
          name="decision"
          value="approve"
          className={`${button} bg-[#24251f] text-[#fffefb] hover:bg-[#3d4535]`}
        >
          Approve
        </button>
        <button
          type="submit"
          name="decision"
          value="deny"
          className={`${button} border-[1.5px] border-current hover:bg-[#f4f5ee]`}
        >
          Deny
        </button>
      </form>
    </Shell>
  );
};

export default AuthorizePage;
