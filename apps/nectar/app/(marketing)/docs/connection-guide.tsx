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
    <div className="rounded-3xl border border-stone-200 bg-stone-50 p-5 sm:p-7">
      <fieldset className="mb-7 flex flex-wrap gap-2" aria-label="MCP client">
        {clients.map((client) => (
          <button
            key={client.id}
            type="button"
            aria-pressed={selected.id === client.id}
            onClick={() => {
              setSelected(client);
              setMessage("");
            }}
            className={`inline-flex items-center gap-2 rounded-full border px-4 py-3 text-sm transition-colors ${selected.id === client.id ? "border-stone-800 bg-white text-stone-950 shadow-sm" : "border-transparent text-stone-500 hover:bg-white"}`}
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
      <p className="text-xs font-semibold tracking-wide text-stone-600">
        HIVE MCP URL
      </p>
      <code className="mt-2 mb-5 block overflow-x-auto rounded-xl border border-stone-200 bg-white px-4 py-3 text-sm">
        {endpoint}
      </code>
      <p className="mb-5 text-sm leading-6 text-stone-500">
        Connect directly to Hive over HTTPS from your agent. Create an API key
        in Account settings → API keys, then set it as HIVE_API_KEY before
        launching your client. Requests authenticate with Authorization: Bearer
        YOUR_HIVE_API_KEY. Keep the key in your environment or private client
        config.
      </p>
      <pre className="mb-5 overflow-x-auto rounded-xl border border-stone-200 bg-white px-4 py-3 text-sm">
        <code>export HIVE_API_KEY=&apos;YOUR_HIVE_API_KEY&apos;</code>
      </pre>
      <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
        <span className="text-sm font-medium text-stone-600">
          {selected.file}
        </span>
        <button
          type="button"
          onClick={() => {
            void copy();
          }}
          className="surface-dark rounded-full px-4 py-2 text-xs text-white"
        >
          Copy config
        </button>
      </div>
      <pre
        className="overflow-x-auto rounded-2xl bg-stone-900 p-5 text-xs leading-6 text-stone-100"
        aria-label={`${selected.name} MCP configuration`}
      >
        <code>{code}</code>
      </pre>
      <output className="mt-3 block min-h-5 text-xs text-lime-800">
        {message}
      </output>
      <p className="mt-2 text-sm leading-6 text-stone-500">{selected.note}</p>
      <a
        href={selected.source}
        target="_blank"
        rel="noreferrer"
        className="mt-4 inline-block text-sm underline underline-offset-4"
      >
        Official {selected.name} setup ↗
      </a>
    </div>
  );
};
