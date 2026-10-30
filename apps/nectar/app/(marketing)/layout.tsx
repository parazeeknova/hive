import type { ReactNode } from "react";

import { session } from "../../lib/server/session";
import Footer from "./footer";
import Navbar from "./navbar";

const MarketingLayout = async ({
  children,
}: Readonly<{ children: ReactNode }>) => {
  const account = await session();
  return (
    <div className="bg-white">
      <a
        href="#main"
        className="fixed -top-20 left-5 z-50 rounded-xl bg-stone-800 px-5 py-3 text-white focus:top-4"
      >
        Skip to content
      </a>
      <Navbar username={account?.name ?? null} />
      <main id="main">{children}</main>
      <Footer />
    </div>
  );
};

export default MarketingLayout;
