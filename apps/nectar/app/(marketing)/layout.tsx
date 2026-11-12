import type { ReactNode } from "react";

import { session } from "../../lib/server/session";
import MarketingShell from "./landing-shell";

const MarketingLayout = async ({
  children,
}: Readonly<{ children: ReactNode }>) => {
  const account = await session();
  return (
    <MarketingShell username={account?.name ?? null}>{children}</MarketingShell>
  );
};

export default MarketingLayout;
