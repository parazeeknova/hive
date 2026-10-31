"use client";
import Image from "next/image";
import Link from "next/link";
import { useState } from "react";

import { Icon } from "../../components/icon";

import "./mcp-section.css";

export const clients = [
  {
    command:
      // oxlint-disable-next-line no-template-curly-in-string -- Claude resolves this stored environment reference.
      'claude mcp add-json --scope project hive \'{"type":"http","url":"https://hiveway.itssingularity.com/mcp","headers":{"Authorization":"Bearer ${HIVE_API_KEY}"}}\'',
    id: "claude",
    name: "Claude Code",
  },
  {
    command:
      "codex mcp add hive --url https://hiveway.itssingularity.com/mcp --bearer-token-env-var HIVE_API_KEY",
    id: "codex",
    name: "Codex",
  },
  {
    command:
      "opencode mcp add hive --url https://hiveway.itssingularity.com/mcp --header 'Authorization=Bearer {env:HIVE_API_KEY}' --global",
    id: "opencode",
    name: "OpenCode",
  },
];
const McpSection = () => {
  const [selected, setSelected] = useState(0);
  const [copied, setCopied] = useState(false);
  const client = clients[selected] ?? clients[0];
  return (
    <section className="hive-mcp" aria-labelledby="hive-mcp-heading">
      <div className="hive-mcp-copy">
        <p className="hive-mcp-eyebrow">YOUR MODEL. A SHARED TOOLBOX.</p>
        <h2 id="hive-mcp-heading">
          Give your agent
          <br />a head start.
        </h2>
        <p>
          Hive is a library of reusable tools your AI can discover and call
          through MCP. Find a capability, give it input, and bring the result
          straight back to your work.
        </p>
        <div className="hive-mcp-points">
          <div>
            <span>01</span>
            <p>
              <strong>Connect once.</strong> Use a scoped API key in your
              preferred coding harness.
            </p>
          </div>
          <div>
            <span>02</span>
            <p>
              <strong>Reuse what works.</strong> Call a published tool instead
              of rebuilding the same logic.
            </p>
          </div>
          <div>
            <span>03</span>
            <p>
              <strong>Build what is missing.</strong> Ask Hive to generate, test
              and publish a pure JSON tool for next time.
            </p>
          </div>
        </div>
        <Link href="/docs#connect">
          Find your integration <Icon name="arrow" size={15} />
        </Link>
      </div>
      <div className="hive-mcp-console">
        <div className="hive-mcp-console-top">
          <span className="hive-mcp-status" /> CONNECTED TO YOUR WORKFLOW{" "}
          <Icon name="bolt" size={15} />
        </div>
        <div
          className="hive-mcp-clients"
          aria-label="Choose your coding harness"
        >
          {clients.map((item, index) => (
            <button
              type="button"
              key={item.id}
              aria-pressed={selected === index}
              onClick={() => {
                setSelected(index);
                setCopied(false);
              }}
            >
              <Image
                src={`/clients/${item.id}.svg`}
                alt=""
                width={23}
                height={23}
              />
              {item.name}
            </button>
          ))}
        </div>
        <div className="hive-mcp-connection">
          <span />
          <div>
            <strong>hive</strong>
            <small>Remote MCP · scoped API key</small>
          </div>
          <span className="hive-mcp-badge">HTTP</span>
        </div>
        <div className="hive-mcp-command">
          <span>$ connect hive</span>
          <p className="mt-3 text-[10px] leading-5 text-stone-400">
            Export HIVE_API_KEY in your shell before launching {client?.name}.
            The command stores a reference to it.
          </p>
          <pre>
            <code>{client?.command}</code>
          </pre>
          <button
            type="button"
            onClick={async () => {
              try {
                if (client) {
                  await navigator.clipboard.writeText(client.command);
                  setCopied(true);
                }
              } catch {
                setCopied(false);
              }
            }}
            aria-label="Copy connection command"
          >
            <Icon name={copied ? "check" : "copy"} size={15} />
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
        <div className="hive-mcp-route">
          <span>YOUR TASK</span>
          <Icon name="arrow" size={13} />
          <span>YOUR AGENT</span>
          <Icon name="arrow" size={13} />
          <strong>HIVE</strong>
        </div>
        <div className="hive-mcp-models">
          <span>GPT · Claude · Muse Spark · DeepSeek</span>
          <p>
            A capable model and an MCP-enabled harness. The same tool API,
            whichever you choose.
          </p>
        </div>
      </div>
    </section>
  );
};
export default McpSection;
