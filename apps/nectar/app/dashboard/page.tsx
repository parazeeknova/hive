import { redirect } from "next/navigation";
import { Suspense } from "react";

import { dashboardDestination } from "../../lib/auth-redirect";
import { privateMetadata } from "../../lib/seo";
import { session } from "../../lib/server/session";
import Workspace from "./workspace";

export const metadata = { ...privateMetadata, title: "The workspace — Hive" };

const DashboardPage = async ({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) => {
  if (!(await session())) {
    const next = dashboardDestination(await searchParams);
    redirect(`/sign-in?${new URLSearchParams({ next })}`);
  }
  return (
    <Suspense
      fallback={
        <div className="mx-auto flex flex-col items-center gap-3 px-6 py-20 text-center text-sm text-stone-500">
          Opening the hive…
        </div>
      }
    >
      <Workspace />
    </Suspense>
  );
};

export default DashboardPage;
