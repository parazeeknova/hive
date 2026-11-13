import { redirect } from "next/navigation";
import { Suspense } from "react";

import { LoadingState } from "../../components/spinner";
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
    <Suspense fallback={<LoadingState screen label="Opening the hive…" />}>
      <Workspace />
    </Suspense>
  );
};

export default DashboardPage;
