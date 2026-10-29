import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  description:
    "Hive is an agent-native capability network. Nectar is its web app.",
  title: "Hive | Nectar",
};

const RootLayout = ({ children }: Readonly<{ children: ReactNode }>) => (
  <html lang="en">
    <body>{children}</body>
  </html>
);

export default RootLayout;
