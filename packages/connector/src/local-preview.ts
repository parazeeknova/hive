import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import type { CallToolResult, Tool } from "@modelcontextprotocol/client";

import type { HiveRpc } from "./hive-rpc";

export const previewTools: Tool[] = [
  {
    annotations: {
      destructiveHint: false,
      openWorldHint: true,
      readOnlyHint: false,
    },
    description:
      "Download up to 5 actual templr template/demo HTML files to this user's computer and serve clickable local previews. Use after showing real get_preview images. Returns localhost URLs and actual local file paths. Files stay on the user's computer; never returns a VPS gallery URL.",
    inputSchema: {
      additionalProperties: false,
      properties: {
        kind: { enum: ["template", "demo"], type: "string" },
        slugs: {
          items: { type: "string" },
          maxItems: 5,
          minItems: 1,
          type: "array",
        },
      },
      required: ["slugs"],
      type: "object",
    },
    name: "preview_templates",
  },
  {
    annotations: { destructiveHint: false, readOnlyHint: false },
    description:
      "Stop this computer's local template preview server. Downloaded HTML files are kept for the user.",
    inputSchema: {
      additionalProperties: false,
      properties: {},
      type: "object",
    },
    name: "stop_preview",
  },
];
const SLUG = /^[a-z0-9]+(?:-[a-z0-9]+)*$/u;
type PreviewKind = "template" | "demo";
/** Validate untrusted preview arguments before any network or filesystem work. */
const previewRequest = (args: Record<string, unknown>) => {
  const { slugs } = args;
  if (
    !Array.isArray(slugs) ||
    slugs.length < 1 ||
    slugs.length > 5 ||
    !slugs.every((slug) => typeof slug === "string" && SLUG.test(slug))
  ) {
    throw new TypeError("Provide 1-5 valid template slugs");
  }
  const kind = args.kind ?? "template";
  if (kind !== "template" && kind !== "demo") {
    throw new TypeError("Unsupported preview kind");
  }
  return { kind: kind as PreviewKind, slugs: [...new Set(slugs as string[])] };
};
const BASE_HEADERS = {
  "Cache-Control": "no-store",
  "Referrer-Policy": "no-referrer",
  "X-Content-Type-Options": "nosniff",
};
const notFound = () => new Response("Not found", { status: 404 });
export class LocalPreviews {
  private server: ReturnType<typeof Bun.serve> | undefined;
  private readonly files = new Map<string, string>();
  private readonly token = crypto.randomUUID().replaceAll("-", "");
  private directory: string | undefined;
  private readonly rpc: Pick<HiveRpc, "tool">;
  private readonly cacheRoot: string;
  constructor(
    rpc: Pick<HiveRpc, "tool">,
    cacheRoot = path.join(
      process.env.XDG_CACHE_HOME ?? path.join(os.homedir(), ".cache"),
      "hive",
      "previews"
    )
  ) {
    this.rpc = rpc;
    this.cacheRoot = cacheRoot;
  }
  async call(
    name: string,
    args: Record<string, unknown>
  ): Promise<CallToolResult> {
    if (name === "stop_preview") {
      this.close();
      return {
        content: [
          {
            text: "Local preview server stopped. Downloaded HTML files were kept.",
            type: "text",
          },
        ],
      };
    }
    if (name !== "preview_templates") {
      throw new TypeError("Unknown local preview tool");
    }
    const { kind, slugs } = previewRequest(args);
    await mkdir(this.cacheRoot, { mode: 0o700, recursive: true });
    this.directory ??= await mkdtemp(path.join(this.cacheRoot, "session-"));
    const entries: { slug: string; file: string; url: string }[] = [];
    // Download sequentially: at most five bounded reads, one upstream call at a time.
    for (const slug of slugs) {
      // oxlint-disable-next-line no-await-in-loop -- Sequential reads bound upstream load.
      const file = await this.download(kind, slug, this.directory);
      this.start();
      entries.push({
        file,
        slug,
        url: `http://127.0.0.1:${this.server?.port}/${this.token}/${kind}/${slug}`,
      });
    }
    const content = {
      directory: this.directory,
      gallery_url: `http://127.0.0.1:${this.server?.port}/${this.token}/`,
      local: true,
      note: "These URLs are served on the computer running Hive Local. Keep the agent/client running while viewing them.",
      previews: entries,
    };
    return {
      content: [{ text: JSON.stringify(content), type: "text" }],
      structuredContent: content,
    };
  }
  close() {
    this.server?.stop(true);
    this.server = undefined;
  }
  private async download(kind: PreviewKind, slug: string, directory: string) {
    const route = `${kind}/${slug}`;
    const existing = this.files.get(route);
    if (existing) {
      return existing;
    }
    if (this.files.size >= 25) {
      throw new Error(
        "Local preview session limit reached; restart Hive Local for another session"
      );
    }
    const tool = kind === "template" ? "read_template_file" : "read_demo_file";
    // The readable alias resolves to the caller's own templr connection, or to
    // Hive's shared read-only templr when the caller does not run one.
    const result = await this.rpc.tool(`mcp_templr__${tool}`, {
      file: "index.html",
      slug,
    });
    if (result.isError) {
      // oxlint-disable-next-line unicorn/prefer-type-error -- isError is an MCP result flag, not a type check.
      throw new Error(`Cannot download ${slug}; no files were executed`);
    }
    const html = result.content
      .filter((block) => block.type === "text")
      .map((block) => block.text)
      .join("\n");
    if (
      !/^\s*(?:<!doctype\s+html|<html|<!--)/iu.test(html) ||
      !/<html[\s>]/iu.test(html) ||
      Buffer.byteLength(html, "utf-8") > 524_288
    ) {
      throw new Error("Preview is not a bounded HTML document");
    }
    const file = path.join(directory, `${kind}-${slug}.html`);
    await writeFile(file, html, { flag: "wx", mode: 0o600 });
    this.files.set(route, file);
    return file;
  }
  private serve(request: Request) {
    const url = new URL(request.url);
    if (
      request.method !== "GET" ||
      !["127.0.0.1", "localhost"].includes(url.hostname) ||
      !url.pathname.startsWith(`/${this.token}/`)
    ) {
      return notFound();
    }
    const origin = request.headers.get("origin");
    if (origin && origin !== url.origin) {
      return new Response("Forbidden", { status: 403 });
    }
    const route = url.pathname.slice(this.token.length + 2);
    const file = this.files.get(route);
    if (file) {
      return new Response(Bun.file(file), {
        headers: {
          ...BASE_HEADERS,
          "Content-Type": "text/html; charset=utf-8",
        },
      });
    }
    if (route) {
      return notFound();
    }
    const links = [...this.files.keys()]
      .map(
        (key) =>
          `<li><a href="/${this.token}/${key}" target="_blank" rel="noopener noreferrer">${key.split("/")[1]}</a></li>`
      )
      .join("");
    return new Response(
      `<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Hive local previews</title><style>body{font:16px system-ui;max-width:760px;margin:48px auto;padding:0 24px;color:#222}li{margin:16px 0}a{color:#285c32}</style></head><body><h1>Your template previews</h1><p>These HTML files are stored and served on your computer.</p><ul>${links}</ul></body></html>`,
      {
        headers: {
          ...BASE_HEADERS,
          "Content-Security-Policy":
            "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'",
          "Content-Type": "text/html; charset=utf-8",
        },
      }
    );
  }
  private start() {
    if (!this.server) {
      this.server = Bun.serve({
        fetch: (request) => this.serve(request),
        hostname: "127.0.0.1",
        port: 0,
      });
    }
  }
}
