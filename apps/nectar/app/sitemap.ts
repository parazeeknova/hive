import type { MetadataRoute } from "next";

import { indexingEnabled, siteUrl } from "../lib/seo";

const sitemap = (): MetadataRoute.Sitemap =>
  indexingEnabled
    ? [
        { changeFrequency: "weekly" as const, path: "/", priority: 1 },
        { changeFrequency: "weekly" as const, path: "/docs", priority: 0.8 },
      ].map(({ path, ...entry }) => ({
        ...entry,
        lastModified: new Date(),
        url: new URL(path, siteUrl).href,
      }))
    : [];

export default sitemap;
