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
  <html className={fontVariables} lang="en">
    <body>
      <Providers>{children}</Providers>
    </body>
  </html>
);

export default RootLayout;
