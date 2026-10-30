import type { Metadata } from "next";

export const siteUrl = new URL(
  process.env.HIVE_SITE_URL ?? "https://hive.itssingularity.com"
);
if (
  siteUrl.pathname !== "/" ||
  siteUrl.search ||
  siteUrl.hash ||
  siteUrl.username ||
  siteUrl.password
) {
  throw new Error(
    "HIVE_SITE_URL must be an origin without a path, query, or credentials"
  );
}
export const indexingEnabled =
  siteUrl.protocol === "https:" &&
  !["localhost", "127.0.0.1", "[::1]"].includes(siteUrl.hostname) &&
  process.env.VERCEL_ENV !== "preview" &&
  process.env.HIVE_SEO_NOINDEX !== "true";

const description =
  "Discover and run reusable AI agent tools on Hive. Try free capabilities with GitHub, connect your agent through MCP, or publish your own Wasm tools.";

export const publicMetadata = (
  title: string,
  path: string,
  summary = description
): Metadata => ({
  alternates: { canonical: new URL(path, siteUrl).href },
  description: summary,
  openGraph: {
    description: summary,
    images: [
      {
        alt: "Hive — Good work, together.",
        height: 630,
        url: "/og",
        width: 1200,
      },
    ],
    locale: "en_US",
    siteName: "Hive",
    title,
    type: "website",
    url: new URL(path, siteUrl).href,
  },
  title: { absolute: title },
  twitter: {
    card: "summary_large_image",
    description: summary,
    images: [{ alt: "Hive — Good work, together.", url: "/og" }],
    title,
  },
});

export const privateMetadata: Metadata = {
  robots: { follow: false, index: false },
};

export const websiteSchema = {
  "@context": "https://schema.org",
  "@type": "WebSite",
  description,
  inLanguage: "en",
  name: "Hive",
  url: siteUrl.href,
};
