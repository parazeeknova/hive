import Link from "next/link";
import { redirect } from "next/navigation";

import { dashboardReturnPath } from "../../lib/auth-redirect";
import { session } from "../../lib/server/session";

export { privateMetadata as metadata } from "../../lib/seo";

const Login = async ({
  searchParams,
}: {
  searchParams: Promise<{ error?: string; next?: string }>;
}) => {
  const params = await searchParams;
  const next = dashboardReturnPath(params.next);
  if (await session()) {
    redirect(next);
  }
  const { error } = params;
  return (
    <main className="mx-auto max-w-md px-6 py-24">
      <h1 className="font-display text-3xl">Welcome to Hive</h1>
      <p className="my-6">
        Sign in or create your account with GitHub to access capabilities,
        artifacts, and execution results.
      </p>
      {error && <p role="alert">GitHub sign-in failed. Please try again.</p>}
      <Link
        prefetch={false}
        className="ws-button inline-flex min-h-[38px] items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
        href={`/api/auth/github?${new URLSearchParams({ next })}`}
      >
        Continue with GitHub
      </Link>
      <p className="my-6">
        <Link href="/">Back to Hive</Link>
      </p>
    </main>
  );
};

export default Login;
