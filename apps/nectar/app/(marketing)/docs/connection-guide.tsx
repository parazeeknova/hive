"use client";

import Image from "next/image";
import { useState } from "react";

const endpoint = "https://hiveway.itssingularity.com/mcp";

const clients = [
  {
    file: ".mcp.json",
    id: "claude",
    name: "Claude Code",
    note: "Merge this server into your project config, then approve it in /mcp. Export HIVE_API_KEY before starting Claude Code.",
    source: "https://code.claude.com/docs/en/mcp",
  },
  {
    file: "opencode.json",
    id: "opencode",
    name: "OpenCode",
    note: "This is the documented JSON config. OpenCode 2.x also supports: opencode mcp add hive --url https://hiveway.itssingularity.com/mcp --header 'Authorization=Bearer {env:HIVE_API_KEY}' --global. Export HIVE_API_KEY before starting OpenCode. Confirm with opencode mcp list.",
    source: "https://opencode.ai/v2/docs/mcp-servers",
  },
  {
    file: "~/.codex/config.toml",
    id: "codex",
    name: "Codex",
    note: "Export HIVE_API_KEY before launching Codex, then inspect the connection with codex mcp list.",
    source: "https://developers.openai.com/codex/mcp",
  },
  {
    file: "~/.pi/agent/mcp.json",
    id: "pi",
    name: "Pi",
    note: "Pi 1.x supports MCP directly. Export HIVE_API_KEY, run pi mcp list, then /reload in an existing session.",
    source: "https://pi.dev/docs/latest/mcp",
  },
  {
    file: "MCP Servers → Manage → View raw config",
    id: "antigravity",
    name: "Antigravity",
    note: "Merge this entry in the raw MCP config. Replace YOUR_HIVE_API_KEY with a scoped credential and keep that file private. Refresh the server list after saving.",
    source: "https://antigravity.google/docs/mcp",
  },
] as const;
const snippet = (id: string) => {
  if (id === "codex") {
    return `[mcp_servers.hive]\nurl = ${JSON.stringify(endpoint)}\nbearer_token_env_var = "HIVE_API_KEY"\ntool_timeout_sec = 180`;
  }
  let config: unknown;
  if (id === "opencode") {
    config = {
      $schema: "https://opencode.ai/config.json",
      mcp: {
        servers: {
          hive: {
            headers: { Authorization: "Bearer {env:HIVE_API_KEY}" },
            oauth: false,
            type: "remote",
            url: endpoint,
          },
        },
      },
    };
  } else if (id === "antigravity") {
    config = {
      mcpServers: {
        hive: {
          headers: { Authorization: "Bearer YOUR_HIVE_API_KEY" },
          serverUrl: endpoint,
          timeoutSeconds: 180,
        },
      },
    };
  } else {
    config = {
      mcpServers: {
        hive: {
          ...(id === "claude" ? { type: "http" } : {}),
          headers: { Authorization: ["Bearer $", "{HIVE_API_KEY}"].join("") },
          url: endpoint,
        },
      },
    };
  }
  return JSON.stringify(config, null, 2);
};
export const ConnectionGuide = () => {
  const [selected, setSelected] = useState<(typeof clients)[number]>(
    clients[0]
  );
  const [message, setMessage] = useState("");
  const code = snippet(selected.id);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setMessage("Configuration copied.");
    } catch {
      setMessage(
        "Clipboard unavailable. Select and copy the configuration below."
      );
    }
  };
  return (
    <div className="flex min-w-0 flex-col gap-5 rounded-[var(--hive-radius)] bg-[#eeeee7] p-4 sm:p-7">
      <fieldset
        className="m-0 flex min-w-0 flex-wrap gap-2 border-0 p-0"
        aria-label="MCP client"
      >
        {clients.map((client) => (
          <button
            key={client.id}
            type="button"
            aria-pressed={selected.id === client.id}
            onClick={() => {
              setSelected(client);
              setMessage("");
            }}
            className={
              "cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45] " +
              `inline-flex min-h-[42px] items-center gap-2 rounded-full border-[1.5px] px-4 text-[15px] font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#24251f] ${selected.id === client.id ? "border-solid border-[#24251f] bg-[#fffefb] text-[#24251f]" : "border-dashed border-[#24251f]/35 text-[#3d4535] hover:border-[#24251f] hover:bg-[#fffefb]"}`
            }
          >
            <Image
              src={`/clients/${client.id}.svg`}
              width={22}
              height={22}
              alt=""
              unoptimized
              className="size-[22px] object-contain"
            />
            {client.name}
          </button>
        ))}
      </fieldset>
      <div className="flex flex-col gap-2">
        <p className="hive-text-4 text-[#727268]">HIVE MCP URL</p>
        <code className="block overflow-x-auto rounded-xl bg-[#fffefb] px-4 py-3 font-mono text-[14px]">
          {endpoint}
        </code>
      </div>
      <p className="text-[15px] leading-relaxed text-[#3d4535]">
        Connect directly to Hive over HTTPS from your agent. Create an API key
        in Account settings → API keys, then set it as HIVE_API_KEY before
        launching your client. Requests authenticate with Authorization: Bearer
        YOUR_HIVE_API_KEY. Keep the key in your environment or private client
        config.
      </p>
      <pre className="m-0 overflow-x-auto rounded-xl bg-[#fffefb] px-4 py-3 font-mono text-[14px]">
        <code>export HIVE_API_KEY=&apos;YOUR_HIVE_API_KEY&apos;</code>
      </pre>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <span className="font-mono text-[14px] font-medium text-[#3d4535]">
          {selected.file}
        </span>
        <button
          type="button"
          onClick={() => {
            void copy();
          }}
          className="inline-flex min-h-[38px] cursor-pointer items-center rounded-full px-4 text-[14px] font-medium text-white shadow-[inset_0_1px_2px_rgb(255_255_255_/_0.2),_inset_0_-2px_4px_rgb(0_0_0_/_0.4),_inset_1px_0_3px_rgb(255_255_255_/_0.045)] transition-[filter] [background:radial-gradient(ellipse_at_50%_0%,_#ffffff0c,_transparent_70%)_padding-box,_linear-gradient(_180deg,_#333_0%,_#222_24%,_#151515_58%,_#101010_80%,_#1b1b1b_100%_)_padding-box,_linear-gradient(180deg,_#505050,_#171717_65%,_#080808)_border-box] [border:1px_solid_transparent] hover:brightness-110 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#24251f] disabled:cursor-not-allowed disabled:opacity-[0.45]"
        >
          Copy config
        </button>
      </div>
      <pre
        className="m-0 overflow-x-auto rounded-2xl bg-[#24251f] p-5 font-mono text-[13px] leading-6 text-[#fffefb]"
        aria-label={`${selected.name} MCP configuration`}
      >
        <code>{code}</code>
      </pre>
      <output className="block text-[13px] text-[#3d4535] empty:hidden">
        {message}
      </output>
      <p className="text-[15px] leading-relaxed text-[#727268]">
        {selected.note}
      </p>
      <a
        href={selected.source}
        target="_blank"
        rel="noreferrer"
        className="hive-link self-start text-[15px]"
      >
        Official {selected.name} setup ↗
      </a>
    </div>
  );
};
