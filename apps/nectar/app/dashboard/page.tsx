import { Suspense } from "react";

import Workspace from "./workspace";

export const metadata = { title: "The workspace — Hive" };

const DashboardPage = () => (
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

export default DashboardPage;
