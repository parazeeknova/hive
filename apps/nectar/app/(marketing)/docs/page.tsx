import Image from "next/image";
import Link from "next/link";
import type { ReactNode } from "react";

import { JsonLd } from "../../../components/json-ld";
import { docsSchema, publicMetadata } from "../../../lib/seo";
import { LandingButton } from "../landing-shell";
import { ConnectionGuide } from "./connection-guide";

export const metadata = publicMetadata(
  "Connect your AI agent with MCP | Hive docs",
  "/docs",
  "Connect Claude Code, OpenCode, Codex, Pi, and Antigravity to Hive with scoped MCP API keys. Run free tools without a wallet and publish reusable Wasm capabilities."
);

// The landing stylesheet resets heading and paragraph margins outside Tailwind's
// layers, so this page spaces content with flex/grid gaps instead of margins.
const contents = [
  ["docs-connect", "Connect your agent"],
  ["docs-credentials", "Get a credential"],
  ["docs-workflow", "Your first invocation"],
  ["docs-external-mcps", "Connect external MCPs"],
  ["docs-hive-local", "Local previews & your keys"],
  ["docs-scaffold", "Start a project"],
  ["docs-build", "Build with Hive"],
  ["docs-publish", "Publish an artifact"],
  ["docs-boundaries", "Payments & limits"],
] as const;
const steps: ReactNode[] = [
  <>
    Ask your agent to use <code>search_tools</code> (or{" "}
    <code>search_capabilities</code> to browse the catalog) for the task at
    hand.
  </>,
  <>
    Inspect the input schema, runtime and atomic-unit price before invoking the
    selected <code>hive_…</code> tool with a fresh UUID{" "}
    <code>idempotency_key</code>.
  </>,
  <>
    Free tools execute directly. Paid tools return a quote and x402
    requirements. Your wallet client signs that specific payment; never send a
    private key.
  </>,
  <>
    Read the receipt. Retry an uncertain call with the same key and input. A new
    key creates a new invocation.
  </>,
];

const Section = ({
  id,
  index,
  title,
  children,
}: {
  id: string;
  index: number;
  title: string;
  children: ReactNode;
}) => (
  <section
    id={id}
    aria-labelledby={`${id}-title`}
    className="grid min-w-0 scroll-mt-28 grid-cols-[minmax(0,1fr)] content-start gap-5 border-t-[1.5px] border-dashed border-[#24251f]/20 pt-9 first:border-t-0 first:pt-0"
  >
    <div className="flex flex-col gap-2">
      <span className="hive-text-4 text-[#727268]">
        {String(index).padStart(2, "0")}
      </span>
      <h2 id={`${id}-title`} className="hive-text-6">
        {title}
      </h2>
    </div>
    <div className="flex min-w-0 flex-col gap-4 text-[16px] leading-[1.7] text-[#3d4535] [&_li_code]:rounded-md [&_li_code]:bg-[#eeeee7] [&_li_code]:px-1.5 [&_li_code]:py-0.5 [&_li_code]:font-mono [&_li_code]:text-[0.86em] [&_p_code]:rounded-md [&_p_code]:bg-[#eeeee7] [&_p_code]:px-1.5 [&_p_code]:py-0.5 [&_p_code]:font-mono [&_p_code]:text-[0.86em] [&_p_code]:[overflow-wrap:anywhere] [&>p]:max-w-[46em]">
      {children}
    </div>
  </section>
);

const Docs = () => (
  <>
    <JsonLd data={docsSchema} />
    <div className="flex flex-col gap-[var(--hive-gutter)] p-[var(--hive-gutter)] pt-[calc(64px+2*var(--hive-gutter))] max-[860px]:pt-[calc(56px+2*var(--hive-gutter))]">
      <header className="hive-surface relative isolate grid min-h-[min(48svh,440px)] content-between gap-8 overflow-hidden rounded-[var(--hive-radius)] p-[clamp(28px,4.4vw,64px)]">
        <Image
          src="/bee-left.png"
          alt=""
          width={325}
          height={304}
          priority
          className="motion-safe:animate-bee-float pointer-events-none absolute top-[clamp(20px,4vw,56px)] right-[clamp(20px,5vw,80px)] -z-1 h-auto w-[clamp(84px,11vw,168px)] rotate-6"
        />
        <p className="hive-text-4">THE BUILDER’S FIELD GUIDE</p>
        <div className="flex flex-col gap-6">
          <h1 className="hive-text-2 max-w-[11em] pr-[clamp(64px,10vw,160px)]">
            A head start for every agent.
          </h1>
          <div className="flex flex-wrap items-end justify-between gap-6">
            <p className="hive-text-5 max-w-[30em]">
              Connect to the hive. Discover a capability, pass it your input,
              and bring the result back to the work you’re already doing.
            </p>
            <div className="flex flex-wrap items-center gap-5">
              <LandingButton href="/dashboard?view=settings&tab=api-keys">
                Create an API key{" "}
                <span className="hive-inline-arrow" aria-hidden="true" />
              </LandingButton>
              <a className="hive-link" href="#docs-connect">
                Connect your agent
              </a>
            </div>
          </div>
        </div>
      </header>
      <div className="grid grid-cols-[minmax(220px,280px)_minmax(0,1fr)] gap-[var(--hive-gutter)] max-[960px]:grid-cols-[minmax(0,1fr)]">
        <aside className="max-[960px]:sticky max-[960px]:top-[calc(56px+2*var(--hive-gutter))] max-[960px]:z-10">
          <nav
            aria-label="Documentation"
            className="hive-surface-3 sticky top-[calc(64px+2*var(--hive-gutter))] flex flex-col gap-1 rounded-[var(--hive-radius)] p-5 max-[960px]:static max-[960px]:[scrollbar-width:none] max-[960px]:flex-row max-[960px]:gap-2 max-[960px]:overflow-x-auto max-[960px]:p-2"
          >
            <span className="hive-text-4 px-3 pt-1 pb-3 text-[#727268] max-[960px]:hidden">
              On this page
            </span>
            {contents.map(([id, label], index) => (
              <a
                key={id}
                href={`#${id}`}
                className="flex items-baseline gap-3 rounded-full px-3 py-2 text-[15px] leading-tight font-medium no-underline transition-colors hover:bg-[#fffefb] focus-visible:outline-2 focus-visible:outline-[#24251f] max-[960px]:shrink-0 max-[960px]:whitespace-nowrap"
              >
                <span className="text-[12px] text-[#727268] tabular-nums max-[960px]:hidden">
                  {String(index + 1).padStart(2, "0")}
                </span>
                {label}
              </a>
            ))}
          </nav>
        </aside>
        <article className="flex min-w-0 flex-col gap-9 self-start rounded-[var(--hive-radius)] border-[1.5px] border-[#eeeee7] bg-[#fffefb] p-[clamp(20px,3.2vw,44px)]">
          <Section id="docs-connect" index={1} title="Make yourself at home.">
            <p>
              Choose your client for its connection settings. Hive uses
              Streamable HTTP at{" "}
              <code>https://hiveway.itssingularity.com/mcp</code> and your Hive
              API key.
            </p>
            <ConnectionGuide />
          </Section>
          <Section
            id="docs-credentials"
            index={2}
            title="One API key. A scoped connection."
          >
            <p>
              Open{" "}
              <Link
                href="/dashboard?view=settings&tab=api-keys"
                className="hive-link"
              >
                Account settings → API keys
              </Link>{" "}
              and sign in with GitHub to create a named, short-lived key for
              free capabilities and free publishing. Connect a wallet for paid
              invocations or paid publishing. Keep the token private and use
              your client’s environment-variable support where available.
            </p>
            <div className="grid gap-3 sm:grid-cols-2">
              {[
                ["capabilities:read", "Search and read the catalog."],
                ["capabilities:invoke", "Run hosted and connected tools."],
                [
                  "capabilities:publish",
                  "Publish capabilities and connectors.",
                ],
                ["workflows:publish", "Compose and publish workflows."],
              ].map(([scope, use]) => (
                <div
                  key={scope}
                  className="flex flex-col gap-1 rounded-2xl bg-[#eeeee7] p-4"
                >
                  <code className="font-mono text-[14px] font-semibold">
                    {scope}
                  </code>
                  <span className="text-[14px] leading-snug text-[#727268]">
                    {use}
                  </span>
                </div>
              ))}
            </div>
            <p>
              Expiry, revocation, per-call limits and total budgets are enforced
              by Hiveway.
            </p>
          </Section>
          <Section
            id="docs-workflow"
            index={3}
            title="Give useful work a second life."
          >
            <ol className="grid gap-3">
              {steps.map((step, index) => (
                <li
                  // Static content: the order never changes.
                  // oxlint-disable-next-line react/no-array-index-key
                  key={index}
                  className="grid grid-cols-[2.25rem_minmax(0,1fr)] items-start gap-3"
                >
                  <span className="grid size-9 place-items-center rounded-full border-[1.5px] border-dashed border-[#24251f] text-[14px] font-medium">
                    {index + 1}
                  </span>
                  <span className="pt-1">{step}</span>
                </li>
              ))}
            </ol>
            <p className="rounded-2xl bg-[#f8cf60] p-5 text-[15px] leading-relaxed text-[#24251f]">
              Try this: “Search Hive for a capability that matches my task. Show
              me its price and schema, then invoke it with my approval. Keep its
              invocation ID so I can inspect the result.”
            </p>
          </Section>
          <Section
            id="docs-external-mcps"
            index={4}
            title="Bring your own MCP servers."
          >
            <p>
              Connect external stdio or HTTP MCP servers through a private Hive
              connector. The connector runs on your computer and makes outbound
              HTTPS requests, so you do not need a public home-network endpoint.
              Hive routing is free and requires no wallet; upstream services may
              charge separately.
            </p>
            <p>
              The official filesystem MCP is included. Configure explicit
              allowed directories and keep your Hive API key outside them. Your
              agent discovers namespaced tools through its existing Hive
              connection. Connections are private to your account and do not
              become public artifacts.
            </p>
            <p>
              Use the filesystem connector preset to request the current working
              directory. Approval happens in your local terminal and lasts for
              that connector session. Switching projects requires approval for
              the new directory. Headless connectors deny new directories.
            </p>
            <p>
              Publish a free connected_mcp artifact bound to an upstream tool
              name, then invoke it with your own connection_id and arguments.
              Compose up to four owned connected artifacts into a free ordered
              workflow. Wasm artifacts keep their pure sandbox ABI.
            </p>
            <p>
              Hive asks connected agents to search for capabilities at each new
              subtask, including mid-session. If your harness ignores MCP server
              instructions, add the reusable guidance from the connector
              documentation to its persistent agent instructions.
            </p>
            <a
              className="hive-link self-start"
              href="https://github.com/parazeeknova/hive/tree/main/packages/connector"
            >
              Connector setup and filesystem configuration
            </a>
          </Section>
          <Section
            id="docs-hive-local"
            index={5}
            title="Your tools. Your keys. Your computer."
          >
            <p>
              Hive Local runs alongside your AI client. It keeps Context7 and
              Tinyfish keys on your computer and exposes their tools through
              Hive. Each user connects their own accounts; Hive never shares
              another user&apos;s keys.
            </p>
            <pre className="overflow-x-auto rounded-2xl bg-[#24251f] p-5 font-mono text-[13px] leading-7 text-[#fffefb]">{`git clone https://github.com/parazeeknova/hive.git
cd hive
bun install --frozen-lockfile --ignore-scripts
bun scripts/setup-hive-local.ts --key-file /absolute/path/hive.key --context7-key-file /absolute/path/context7.key --tinyfish-key-file /absolute/path/tinyfish.key`}</pre>
            <p>
              Save your keys in private files outside your project. The setup
              command prints a local MCP command for your client. Provider key
              files are optional. Your Hive key needs read, invoke and publish
              scopes so the helper can discover tools and register its private
              worker connections.
            </p>
            <p>
              Already using OpenCode with these MCPs? Run{" "}
              <code>bun scripts/setup-hive-local.ts --import-opencode</code> to
              migrate the existing keys, preserve a private backup, install
              persistent routing guidance and replace the three direct
              connections with one Hive entry. Reconnect the client once after
              setup.
            </p>
            <div className="grid gap-[var(--hive-gutter)] md:grid-cols-2">
              <div className="flex flex-col gap-3 rounded-2xl bg-[#eeeee7] p-5">
                <h3 className="text-[18px] leading-snug font-medium">
                  Real HTML previews, served locally
                </h3>
                <p className="text-[15px] leading-relaxed">
                  templr is shared by Hive with every account: searching
                  templates and viewing preview images needs no key or setup,
                  and only its read-only tools are shared. Once you choose
                  candidates, the local-preview tool downloads their HTML into
                  your user cache and starts a loopback-only server on the
                  computer running Hive Local. Downloaded HTML remains after the
                  preview server stops.
                </p>
              </div>
              <div className="flex flex-col gap-3 rounded-2xl bg-[#eeeee7] p-5">
                <h3 className="text-[18px] leading-snug font-medium">
                  Discovery that grows with your catalog
                </h3>
                <p className="text-[15px] leading-relaxed">
                  Hive exposes a compact tool list. Agents use search_tools to
                  fetch only relevant schemas, then call_tool to execute them.
                  Newly connected MCPs and user-published artifacts appear
                  without loading every tool into the conversation. Search is
                  bounded and owner-scoped; it grants no new permissions and
                  authorizes no payments. A model may still ignore guidance.
                </p>
              </div>
            </div>
          </Section>
          <Section
            id="docs-scaffold"
            index={6}
            title="Start from any public GitHub starter."
          >
            <p>
              Ask your connected agent to call <code>scaffold_project</code>{" "}
              with <code>repository_url</code>, a full 40-character commit{" "}
              <code>revision</code>, and <code>project_name</code>. The source
              is your choice: Hive has no built-in repository or template
              snapshot. This free tool verifies a bounded public source archive
              and returns its download URL, checksum, and local setup
              instructions.
            </p>
            <p>
              Publish a free capability with runtime repository_scaffold to list
              this operation in Discover and invoke it with normal quotes and
              receipts. Its input contains project_name, repository_url and
              revision. The scaffold_project MCP name remains available for
              existing clients. Your agent downloads and verifies the archive,
              then uses its local filesystem MCP or shell tools to create a new
              directory. Existing directories are preserved, and source package
              names stay unchanged. Source scripts and dependency installation
              are never executed by Hive. Archives are limited to 8 MiB
              compressed, 32 MiB expanded and 2,000 files. Links and unsafe
              paths are rejected by the local installer.
            </p>
            <p>
              From a Hive checkout, run{" "}
              <code>
                bun scripts/scaffold.ts REPOSITORY_URL COMMIT_SHA my-app
              </code>{" "}
              with <code>HIVE_API_KEY</code> set. The installer verifies and
              writes the files and initializes local Git. Creating and pushing a
              GitHub repository uses your local GitHub CLI or GitHub MCP
              authorization separately. Never send the Hive API key to the
              source archive host.
            </p>
          </Section>
          <Section id="docs-build" index={7} title="Ask Hive to build a tool.">
            <p>
              Open Account settings → Capabilities → Build with Hive, or call{" "}
              <code>build_capability</code> over MCP with a new UUID{" "}
              <code>idempotency_key</code>, a name, description, task and one to
              eight examples containing <code>input</code> and{" "}
              <code>expected</code>. Poll <code>get_build</code> with the
              returned <code>build_id</code>. Hive generates Wasm code, compiles
              it and runs every supplied example. When they pass, it publishes a
              free artifact under your account. No wallet needed.
            </p>
            <p>
              These tools process JSON in an isolated runtime without filesystem
              or network access, for example a validator, formatter or
              calculator. Review generated source and test more inputs: passing
              examples is not a correctness guarantee. Builds are asynchronous,
              limited per account and across the server, and may fail without
              publishing. Reuse the same idempotency key and request when
              retrying.
            </p>
            <p>
              Tasks, examples and generated code go to the server&apos;s
              configured model provider. Meta Contributor mode may use this
              content for training. Your Hive API key and the server&apos;s
              model key are never included in generation prompts.
            </p>
          </Section>
          <Section
            id="docs-publish"
            index={8}
            title="Publish something worth reusing."
          >
            <p>
              Use <code>upload_artifact</code> to upload a JSON-in/JSON-out Wasm
              module and its input/output schemas. Then use{" "}
              <code>publish_capability</code> with its artifact ID, runtime{" "}
              <code>wasm</code>, and an atomic price string. Set{" "}
              <code>price: &quot;0&quot;</code> for a free capability. USDC uses
              six decimal places: <code>&quot;10000&quot;</code> is 0.01 USDC.
            </p>
            <p>
              Code and prices are immutable. Publish a new capability to change
              either. You can pause or restore your listing in Account settings;
              previous quotes and receipts stay pinned.
            </p>
          </Section>
          <Section
            id="docs-boundaries"
            index={9}
            title="Clear boundaries. Useful tools."
          >
            <div className="grid gap-[var(--hive-gutter)] md:grid-cols-2">
              <div className="hive-surface-2 flex flex-col gap-3 rounded-2xl p-6">
                <h3 className="text-[18px] leading-snug font-medium">
                  Your agent stays in charge
                </h3>
                <p className="text-[15px] leading-relaxed opacity-85">
                  Call Hive during your own workflow. Hosted Wasm has no
                  filesystem or network access. It cannot call another hosted
                  API from inside the sandbox.
                </p>
              </div>
              <div className="hive-surface-4 flex flex-col gap-3 rounded-2xl p-6">
                <h3 className="text-[18px] leading-snug font-medium">
                  A receipt for each call
                </h3>
                <p className="text-[15px] leading-relaxed opacity-85">
                  x402 payments settle to the creator. Dodo currently supports
                  test checkout and merchant payables. Session budget and
                  earnings history are not a spendable wallet balance.
                </p>
              </div>
            </div>
            <p className="text-[15px] text-[#727268]">
              Client configuration examples follow the official documentation
              linked below each example. OpenCode has been tested live with
              Hive; the other examples are documentation-verified, not claims of
              end-to-end testing in every client.
            </p>
          </Section>
        </article>
      </div>
    </div>
  </>
);
export default Docs;
