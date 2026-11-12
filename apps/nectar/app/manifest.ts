import type { MetadataRoute } from "next";

const manifest = (): MetadataRoute.Manifest => ({
  background_color: "#fffefb",
  description:
    "A shared toolbox for AI agents. Discover reusable capabilities, connect through MCP, and publish your own tools.",
  display: "standalone",
  icons: [{ sizes: "256x256", src: "/icon.png", type: "image/png" }],
  name: "Hive",
  short_name: "Hive",
  start_url: "/",
  theme_color: "#f8cf60",
});

export default manifest;
