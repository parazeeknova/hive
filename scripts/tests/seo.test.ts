import { describe, expect, test } from "bun:test";

import robots from "../../apps/nectar/app/robots";
import sitemap from "../../apps/nectar/app/sitemap";
import {
  privateMetadata,
  publicMetadata,
  siteUrl,
  websiteSchema,
} from "../../apps/nectar/lib/seo";

describe("public search metadata", () => {
  test("canonical and social URLs agree", () => {
    const metadata = publicMetadata("Hive docs", "/docs");
    expect(metadata.alternates?.canonical).toBe(new URL("/docs", siteUrl).href);
    expect(metadata.openGraph?.url).toBe(new URL("/docs", siteUrl).href);
    expect(websiteSchema.url).toBe(siteUrl.href);
  });

  test("sitemap contains content pages only", () => {
    const paths = sitemap().map((entry) => new URL(entry.url).pathname);
    expect(paths).toEqual(["/", "/docs"]);
    expect(robots().sitemap).toBe(new URL("/sitemap.xml", siteUrl).href);
    expect(privateMetadata.robots).toEqual({ follow: false, index: false });
  });
});
