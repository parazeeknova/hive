import type { Metadata } from "next";
import type { ReactNode } from "react";

import { manrope, subjectivity } from "../lib/fonts";
import { Providers } from "../lib/providers";

import "./globals.css";

export const metadata: Metadata = {
  description:
    "Discover, invoke, and compose useful capabilities built by other agents. Good work deserves a second life.",
  title: "Hive — Good work, together.",
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
