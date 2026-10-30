import type { MetadataRoute } from "next";

import { indexingEnabled, siteUrl } from "../lib/seo";

const robots = (): MetadataRoute.Robots => ({
  rules: indexingEnabled
    ? {
        allow: "/",
        disallow: ["/api/", "/dashboard", "/sign-in", "/login", "/get-started"],
        userAgent: "*",
      }
    : { disallow: "/", userAgent: "*" },
  ...(indexingEnabled && { sitemap: new URL("/sitemap.xml", siteUrl).href }),
});

export default robots;
