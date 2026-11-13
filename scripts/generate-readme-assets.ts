/** Rebuild README artwork from the existing Hive mark. Requires rsvg-convert. */
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const target = path.join(root, "docs/assets");
await mkdir(target, { recursive: true });
const scratch = path.join(root, ".hive/readme-artwork");
await mkdir(scratch, { recursive: true });
const fontConfig = path.join(scratch, "fonts.conf");
await writeFile(
  fontConfig,
  `<?xml version="1.0"?><!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd"><fontconfig><dir>${path.join(root, "apps/nectar/app/fonts")}</dir><cachedir>${scratch}</cachedir></fontconfig>`
);
const logoBytes = await readFile(
  path.join(root, "apps/nectar/public/hive-mark.png")
);
const mark = `data:image/png;base64,${logoBytes.toString("base64")}`;
const logo = (x: number, y: number, size: number) =>
  `<image href="${mark}" x="${x}" y="${y}" width="${size}" height="${size}"/>`;
const text = (
  x: number,
  y: number,
  value: string,
  size = 18,
  weight = 400,
  color = "#263c32"
) =>
  `<text x="${x}" y="${y}" font-size="${size}" font-weight="${weight}" fill="${color}">${value}</text>`;
const card = (x: number, y: number, w: number, h: number, fill = "#ffffff") =>
  `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="22" fill="${fill}" stroke="#d9dfcf"/>`;
const line = (d: string, dash = false) =>
  `<path d="${d}" fill="none" stroke="#6d8b73" stroke-width="2" ${dash ? 'stroke-dasharray="7 7"' : ""} marker-end="url(#arrow)"/>`;
const shell = (
  title: string,
  description: string,
  height: number,
  content: string
) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="${height}" viewBox="0 0 1200 ${height}" role="img" aria-labelledby="title description"><title id="title">${title}</title><desc id="description">${description}</desc><defs><marker id="arrow" markerWidth="10" markerHeight="10" refX="8" refY="5" orient="auto"><path d="M1 1 L8 5 L1 9" fill="none" stroke="#6d8b73" stroke-width="1.5"/></marker><radialGradient id="glow"><stop stop-color="#ffe8a8"/><stop offset="1" stop-color="#f8f6ed"/></radialGradient><pattern id="dots" width="24" height="24" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r="1" fill="#c6d1bd" opacity=".45"/></pattern></defs><rect width="1200" height="${height}" rx="28" fill="#f8f6ed"/><rect width="1200" height="${height}" rx="28" fill="url(#dots)"/><g font-family="Manrope,DejaVu Sans,sans-serif">${content}</g></svg>`;

const hero = shell(
  "Hive — one endpoint, more capable agents",
  "Hive connects agents to tools, repository context and reusable workflows through one MCP endpoint.",
  500,
  `
<circle cx="960" cy="230" r="260" fill="url(#glow)"/>
${text(54, 58, "THE CAPABILITY NETWORK FOR AI AGENTS", 13, 700, "#63745f")}
${logo(54, 90, 76)}${text(146, 154, "hive", 80, 800)}${text(326, 154, ".", 80, 800, "#eba623")}
${text(54, 248, "One endpoint.", 52, 800)}
${text(54, 312, "More capable agents.", 52, 800)}
${text(56, 363, "Find the right tool. Connect context. Keep what works.", 19, 400, "#5f7063")}
${card(56, 405, 150, 40, "#e8eddf")}${text(77, 431, "MCP native", 15, 700)}
${card(220, 405, 130, 40, "#e8eddf")}${text(244, 431, "Rust core", 15, 700)}
${card(364, 405, 235, 40, "#fff0bf")}${text(388, 431, "Your tools, together", 15, 700)}
<path d="M864 249 Q786 249 786 143 L866 143 M982 249 L1044 249 M894 291 L894 366 L985 366" stroke="#a9b998" stroke-width="2" fill="none"/>
${card(842, 195, 150, 125, "#fffdf5")}${logo(878, 213, 78)}
${card(841, 91, 290, 80)}${text(869, 125, "Repository context", 19, 700)}${text(869, 151, "Code index + memory", 15, 400, "#63745f")}
${card(1024, 208, 150, 78)}${text(1043, 239, "Workflows", 18, 700)}${text(1043, 264, "Tools in sync", 14, 400, "#63745f")}
${card(862, 353, 270, 80)}${text(889, 387, "Local + shared tools", 19, 700)}${text(889, 414, "One hosted connection", 15, 400, "#63745f")}`
);

const architecture = shell(
  "How Hive connects your agent and tools",
  "Agents call the hosted Hive endpoint. Hive runs native features, routes shared and provider MCP calls, and queues private jobs for a local helper that initiates outbound HTTPS requests.",
  730,
  `
${logo(38, 27, 42)}${text(98, 58, "One connection. Three places to work.", 28, 800)}
${text(40, 96, "Your agent sees one hosted MCP endpoint. Hive routes each call to the right capability.", 17, 400, "#64735e")}
${line("M286 257 L378 257")}${text(295, 235, "MCP", 13, 700)}
${line("M698 222 L796 183")}${line("M698 288 L796 343")}
${card(40, 178, 246, 163)}${text(64, 217, "YOUR AGENT", 12, 800, "#71816b")}${text(64, 258, "Ask in plain language", 19, 700)}${text(64, 290, "Codex · Claude · OpenCode", 15)}${text(64, 316, "Any compatible MCP client", 14, 400, "#64735e")}
${card(378, 156, 320, 220, "#fff0bf")}${logo(404, 175, 62)}${text(482, 218, "hive.", 35, 800)}
${text(405, 262, "Hosted /mcp", 24, 800)}${text(405, 294, "Discovery · routing · scoped access", 15)}${text(405, 322, "Workflow engine · jobs · caching", 15)}${text(405, 350, "Your credential on every step", 14, 400, "#64735e")}
${card(796, 126, 364, 122)}${text(820, 162, "NATIVE HIVE CAPABILITIES", 12, 800, "#71816b")}${text(820, 195, "Code index · repository memory", 18, 700)}${text(820, 224, "Catalog · workflows · WASM artifacts", 15)}
${card(796, 279, 364, 142)}${text(820, 315, "SHARED + PROVIDER MCPs", 12, 800, "#71816b")}${text(820, 347, "Documents · SQL · templates", 18, 700)}${text(820, 376, "Context7 · TinyFish with your keys", 15)}${text(820, 402, "Shared upstreams need provisioning", 13, 400, "#64735e")}
${line("M538 515 L538 386", true)}${text(560, 458, "Outbound HTTPS", 15, 700)}${text(560, 483, "Long-poll for jobs; return results", 14, 400, "#64735e")}
${card(38, 521, 1124, 168, "#e8eddf")}${text(64, 558, "YOUR COMPUTER", 12, 800, "#63745f")}${text(64, 589, "Windows · macOS · Linux", 21, 700)}${text(64, 619, "Approved folders + projects", 15)}${text(64, 651, "No inbound port or public tunnel", 14, 400, "#63745f")}
${card(424, 548, 225, 110)}${logo(443, 567, 35)}${text(491, 592, "Hive Local", 21, 800)}${text(448, 632, "Rust background worker", 14)}
${line("M650 604 L720 604")}${card(724, 548, 412, 110)}${text(748, 582, "Filesystem · FFmpeg · Serena", 19, 700)}${text(748, 612, "Local MarkItDown + DuckDB", 18, 700)}${text(748, 640, "Tool results pass back through Hive", 14, 400, "#64735e")}`
);

const workflows = shell(
  "One prompt, connected context, reusable workflows",
  "A change brief fans out to symbol lookup, references and repository memory, joins their results and can become a private reusable tool after repeated successful runs.",
  480,
  `
${logo(38, 25, 42)}${text(99, 57, "A prompt can become a workflow.", 28, 800)}${text(40, 94, "Example: understand a symbol before changing it. Independent steps run together.", 17, 400, "#64735e")}
${line("M250 252 L285 252 L285 161 L327 161")}${line("M285 252 L327 252")}${line("M285 252 L285 343 L327 343")}
${line("M544 161 L585 161 L585 252 L633 252")}${line("M544 252 L633 252")}${line("M544 343 L585 343 L585 252")}${line("M865 252 L934 252")}
${card(40, 192, 210, 120)}${text(62, 230, "ONE PROMPT", 12, 800, "#71816b")}${text(62, 264, "“What changes if", 19, 700)}${text(62, 290, "I edit this symbol?”", 19, 700)}
${card(327, 124, 217, 74)}${text(347, 155, "Find its definition", 18, 700)}${text(347, 181, "Code index / Serena", 13, 400, "#64735e")}
${card(327, 215, 217, 74)}${text(347, 247, "Find references", 18, 700)}${text(347, 272, "Understand the impact", 13, 400, "#64735e")}
${card(327, 306, 217, 74)}${text(347, 338, "Recall decisions", 18, 700)}${text(347, 363, "Repository memory", 13, 400, "#64735e")}
${card(633, 193, 232, 119, "#fff0bf")}${text(657, 233, "ONE CHANGE BRIEF", 12, 800, "#71816b")}${text(657, 269, "Connected results", 21, 800)}${text(657, 295, "Same caller, checked scopes", 13)}
${card(934, 193, 225, 119)}${text(955, 232, "REPEAT → REUSE", 12, 800, "#71816b")}${text(955, 267, "A private tool", 21, 800)}${text(955, 295, "Learned after successful runs", 13)}
${text(42, 444, "Discover → compose → run → learn. Saved recipes execute only when an agent calls them.", 16, 600, "#63745f")}`
);

await Promise.all(
  Object.entries({
    "hive-architecture": architecture,
    "hive-header": hero,
    "hive-workflows": workflows,
  }).map(async ([name, svg]) => {
    const source = path.join(target, `${name}.svg`);
    await writeFile(source, svg);
    const rendered = Bun.spawn(
      [
        "rsvg-convert",
        "--width",
        "2400",
        "--output",
        path.join(target, `${name}.png`),
        source,
      ],
      {
        env: { ...process.env, FONTCONFIG_FILE: fontConfig },
        stderr: "inherit",
        stdout: "inherit",
      }
    );
    if ((await rendered.exited) !== 0) {
      throw new Error(`Could not render ${name}`);
    }
  })
);
