import type { Metadata } from "next";
import type { ReactNode } from "react";

import { manrope, subjectivity } from "../lib/fonts";
import { Providers } from "../lib/providers";
import { indexingEnabled, siteUrl } from "../lib/seo";

import "./globals.css";

export const metadata: Metadata = {
  applicationName: "Hive",
  metadataBase: siteUrl,
  referrer: "strict-origin-when-cross-origin",
  robots: { follow: indexingEnabled, index: indexingEnabled },
  title: {
    default: "Hive — AI agent tools and MCP capabilities",
    template: "%s | Hive",
  },
};

const fontVariables = [manrope.variable, subjectivity.variable].join(" ");

const RootLayout = ({ children }: Readonly<{ children: ReactNode }>) => (
  <html
    className={`${fontVariables} scroll-pt-[5.5rem] scroll-smooth selection:bg-amber-200 [&_:focus-visible]:outline-2 [&_:focus-visible]:outline-offset-4 [&_:focus-visible]:outline-amber-600 [&_a_svg]:shrink-0 [&_button_svg]:shrink-0`}
    lang="en"
  >
    <body className="font-body bg-stone-50 text-sm text-stone-800 antialiased">
      <Providers>{children}</Providers>
    </body>
  </html>
);

export default RootLayout;
