import type { MetadataRoute } from "next";

import { indexingEnabled, siteUrl } from "../lib/seo";

const sitemap = (): MetadataRoute.Sitemap =>
  indexingEnabled
    ? ["/", "/docs"].map((path) => ({ url: new URL(path, siteUrl).href }))
    : [];

export default sitemap;
