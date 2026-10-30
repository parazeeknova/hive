# Search and sharing

The production origin defaults to `https://hive.itssingularity.com`. Set the server environment variable `HIVE_SITE_URL` to override it. Use an origin without a path, query, or credentials. Set `HIVE_SEO_NOINDEX=true` on staging and preview deployments; Vercel previews are excluded automatically. Do not enable indexing on a development host.

The homepage and `/docs` have unique titles, descriptions, canonical URLs, Open Graph and Twitter metadata. `/og` generates the shared 1200×630 preview using Next.js’s bundled font. The homepage includes WebSite JSON-LD. `/sitemap.xml` lists only the two public content pages. Auth, workspace, redirect, and API routes are excluded; API responses also carry `X-Robots-Tag`.

After deployment, verify the canonical host serves HTTPS and redirects any alternate host to it. Check `/robots.txt`, `/sitemap.xml`, `/og`, and the rendered metadata on both public pages. Verify the domain in Google Search Console and Bing Webmaster Tools, then submit `https://hive.itssingularity.com/sitemap.xml`. Domain verification and search engine submission require access to those accounts and are separate from the code change. Search engines decide when and whether to index pages.

Keep this sitemap limited to public pages with useful server-rendered content. Dashboard search results and private artifacts are not public landing pages. Add any future public capability pages only when they have independent content and stable URLs.
