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
      <h1 className="text-3xl">Welcome to Hive</h1>
      <p className="my-6">
        Sign in or create your account with GitHub to access capabilities,
        artifacts, and execution results.
      </p>
      {error && <p role="alert">GitHub sign-in failed. Please try again.</p>}
      <Link
        prefetch={false}
        className="ws-button"
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
