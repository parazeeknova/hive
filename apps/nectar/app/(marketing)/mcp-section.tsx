"use client";
import Image from "next/image";
import Link from "next/link";
import { useState } from "react";

import { Icon } from "../../components/icon";

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
    <section
      className="hive-mcp mx-auto my-0 grid w-[calc(100%_-_64px)] max-w-[1152px] grid-cols-[1fr_1.08fr] items-center gap-[80px] px-0 pt-[80px] pb-[110px] max-[900px]:max-w-[620px] max-[900px]:grid-cols-[1fr] max-[900px]:gap-[38px] max-[900px]:px-0 max-[900px]:pt-[50px] max-[900px]:pb-[75px] max-[480px]:w-[calc(100%_-_32px)]"
      aria-labelledby="hive-mcp-heading"
    >
      <div className="hive-mcp-copy [&_h2]:mx-0 [&_h2]:mt-5 [&_h2]:mb-6 [&_h2]:text-[clamp(36px,_4.7vw,_58px)] [&_h2]:leading-[1.12] [&_h2]:font-semibold [&_h2]:tracking-[-0.055em] max-[480px]:[&_h2]:text-[40px] max-[900px]:[&_h2_br]:hidden [&>a]:inline-flex [&>a]:items-center [&>a]:gap-2.5 [&>a]:text-[12px] [&>a]:font-semibold [&>a]:text-[#445637] [&>p]:max-w-[420px] [&>p]:text-[15px] [&>p]:leading-[1.85] [&>p]:text-[#77796f] max-[900px]:[&>p]:max-w-[none]">
        <p className="hive-mcp-eyebrow text-[10px]! font-semibold tracking-[0.16em] text-[color:#7c846e]!">
          YOUR MODEL. A SHARED TOOLBOX.
        </p>
        <h2 id="hive-mcp-heading" className="font-display">
          Give your agent
          <br />a head start.
        </h2>
        <p>
          Hive is a library of reusable tools your AI can discover and call
          through MCP. Find a capability, give it input, and bring the result
          straight back to your work.
        </p>
        <div className="hive-mcp-points mx-0 my-[30px] grid gap-[21px] [&_p]:text-[13px] [&_p]:leading-[1.8] [&_p]:text-[#7c7e75] [&_strong]:font-semibold [&_strong]:text-[#333a2c] [&>div]:flex [&>div]:gap-[17px] [&>div>span]:pt-1 [&>div>span]:text-[#b6b9ab] [&>div>span]:[font:11px_var(--font-mono)]">
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
      <div className="hive-mcp-console min-w-0 overflow-hidden rounded-[20px] bg-[#fcfdf9] shadow-[0_18px_70px_#2c3d1610] [border:1px_solid_#dce1d4]">
        <div className="hive-mcp-console-top flex items-center gap-2 px-[23px] py-[18px] text-[9px] tracking-[0.12em] text-[#8b927f] [border-bottom:1px_solid_#e9ecdf] max-[480px]:p-[17px] max-[480px]:text-[8px] [&>svg]:ml-auto">
          <span className="hive-mcp-status h-1.5 w-1.5 rounded-full bg-[#8cae52] shadow-[0_0_0_4px_#8cae5215]" />{" "}
          CONNECTED TO YOUR WORKFLOW <Icon name="bolt" size={15} />
        </div>
        <div
          className="hive-mcp-clients flex gap-[5px] px-[18px] pt-[19px] pb-0 max-[480px]:px-2.5 max-[480px]:pt-[15px] max-[480px]:pb-0 [&_button]:flex [&_button]:min-w-0 [&_button]:[flex:1] [&_button]:cursor-pointer [&_button]:items-center [&_button]:justify-center [&_button]:gap-[7px] [&_button]:rounded-[8px] [&_button]:px-2.5 [&_button]:py-[9px] [&_button]:text-[11px] [&_button]:text-[#848b7d] [&_button]:[background:none] [&_button]:[border:1px_solid_transparent] max-[480px]:[&_button]:gap-[5px] max-[480px]:[&_button]:px-1.5 max-[480px]:[&_button]:py-2 max-[480px]:[&_button]:text-[9px] [&_button[aria-pressed='true']]:[border-color:#dde5cf] [&_button[aria-pressed='true']]:bg-white [&_button[aria-pressed='true']]:text-[#354327] [&_button[aria-pressed='true']]:shadow-[0_2px_5px_#00000004]"
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
              className="cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45]"
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
        <div className="hive-mcp-connection flex items-center gap-2.5 px-[26px] pt-[26px] pb-5 max-[480px]:px-[17px] max-[480px]:py-[23px] [&_small]:mt-[3px] [&_small]:block [&_small]:text-[10px] [&_small]:text-[#969c8d] [&_strong]:text-[15px] [&_strong]:font-semibold [&>span:first-child]:h-[33px] [&>span:first-child]:w-[33px] [&>span:first-child]:rounded-[9px] [&>span:first-child]:bg-[#eaf0dd] [&>span:first-child]:[clip-path:polygon(25%_5%,_75%_5%,_100%_50%,_75%_95%,_25%_95%,_0_50%)]">
          <span />
          <div>
            <strong>hive</strong>
            <small>Remote MCP · scoped API key</small>
          </div>
          <span className="hive-mcp-badge ml-auto rounded-[4px] px-1.5 py-1 text-[#8a947b] [border:1px_solid_#e1e7d6] [font:9px_var(--font-mono)]">
            HTTP
          </span>
        </div>
        <div className="hive-mcp-command mx-6 my-0 rounded-[10px] bg-[#20251b] p-5 text-[#edf2e3] max-[480px]:mx-3.5 max-[480px]:my-0 max-[480px]:p-4 [&_button]:flex [&_button]:cursor-pointer [&_button]:items-center [&_button]:gap-[7px] [&_button]:rounded-[5px] [&_button]:px-2 [&_button]:py-[5px] [&_button]:text-[10px] [&_button]:text-[#b1bf9b] [&_button]:[border:1px_solid_#49513c] [&_pre]:mx-0 [&_pre]:my-[15px] [&_pre]:[overflow-wrap:anywhere] [&_pre]:whitespace-pre-wrap [&_pre]:text-[#e4ebd6] [&_pre]:[font:11px/1.9_var(--font-mono)] [&>span]:text-[#9daa87] [&>span]:[font:10px_var(--font-mono)]">
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
            className="cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45]"
          >
            <Icon name={copied ? "check" : "copy"} size={15} />
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
        <div className="hive-mcp-route flex items-center justify-center gap-[13px] px-[15px] py-[27px] tracking-[0.06em] text-[#a1a78f] [font:8px_var(--font-mono)] max-[480px]:gap-[9px] max-[480px]:text-[7px] [&_strong]:font-semibold [&_strong]:text-[#617d38]">
          <span>YOUR TASK</span>
          <Icon name="arrow" size={13} />
          <span>YOUR AGENT</span>
          <Icon name="arrow" size={13} />
          <strong>HIVE</strong>
        </div>
        <div className="hive-mcp-models px-6 pt-[18px] pb-[23px] [border-top:1px_solid_#e9ecdf] [&_p]:mt-1.5 [&_p]:text-[10px] [&_p]:leading-[1.8] [&_p]:text-[#a0a591] [&>span]:text-[11px] [&>span]:font-medium [&>span]:text-[#666e59]">
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
