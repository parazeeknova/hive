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

export const publisher = {
  "@type": "Organization",
  name: "Singularity Works",
  url: "https://itssingularity.com",
} as const;

export const organizationSchema = {
  "@context": "https://schema.org",
  ...publisher,
  logo: new URL("/icon.png", siteUrl).href,
  sameAs: ["https://github.com/parazeeknova/hive"],
};

export const applicationSchema = {
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  applicationCategory: "DeveloperApplication",
  description,
  name: "Hive",
  offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
  operatingSystem: "Any",
  publisher,
  url: siteUrl.href,
};

/** FAQPage data must mirror the visible answers in plain text. */
export const faqSchema = (items: { question: string; answer: string }[]) => ({
  "@context": "https://schema.org",
  "@type": "FAQPage",
  mainEntity: items.map(({ question, answer }) => ({
    "@type": "Question",
    acceptedAnswer: { "@type": "Answer", text: answer },
    name: question,
  })),
});

export const docsSchema = [
  {
    "@context": "https://schema.org",
    "@type": "TechArticle",
    about: "Connecting AI agents to Hive over MCP",
    description:
      "Connect Claude Code, OpenCode, Codex, Pi, and Antigravity to Hive with scoped MCP API keys, then discover, run and publish reusable capabilities.",
    headline: "A head start for every agent",
    inLanguage: "en",
    mainEntityOfPage: new URL("/docs", siteUrl).href,
    publisher,
  },
  {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [
      { "@type": "ListItem", item: siteUrl.href, name: "Hive", position: 1 },
      {
        "@type": "ListItem",
        item: new URL("/docs", siteUrl).href,
        name: "Docs",
        position: 2,
      },
    ],
  },
];
