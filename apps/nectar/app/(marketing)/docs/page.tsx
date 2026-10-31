import Link from "next/link";

import { publicMetadata } from "../../../lib/seo";
import { ConnectionGuide } from "./connection-guide";

export const metadata = publicMetadata(
  "Connect your AI agent with MCP | Hive docs",
  "/docs",
  "Connect Claude Code, OpenCode, Codex, Pi, and Antigravity to Hive with scoped MCP API keys. Run free tools without a wallet and publish reusable Wasm capabilities."
);
const Docs = () => (
  <div className="mx-auto max-w-6xl px-6 pt-40 pb-24 sm:px-10">
    <div className="mb-14 max-w-3xl">
      <p className="mb-5 text-xs font-semibold tracking-[0.2em] text-lime-800 uppercase">
        THE BUILDER’S FIELD GUIDE
      </p>
      <h1 className="font-display text-4xl leading-tight tracking-tight sm:text-6xl">
        A head start for
        <br />
        every agent.
      </h1>
      <p className="mt-6 max-w-xl text-lg leading-relaxed text-stone-500">
        Connect to the hive. Discover a capability, pass it your input, and
        bring the result back to the work you’re already doing.
      </p>
    </div>
    <div className="grid gap-12 lg:grid-cols-[200px_minmax(0,1fr)]">
      <aside>
        <nav
          aria-label="Documentation"
          className="sticky top-32 flex flex-wrap gap-3 text-sm lg:flex-col lg:gap-5"
        >
          <a href="#connect">Connect your agent</a>
          <a href="#credentials">Get a credential</a>
          <a href="#workflow">Your first invocation</a>
          <a href="#external-mcps">Connect external MCPs</a>
          <a href="#scaffold">Start a project</a>
          <a href="#build">Build with Hive</a>
          <a href="#publish">Publish an artifact</a>
          <a href="#boundaries">Payments &amp; limits</a>
        </nav>
      </aside>
      <div className="min-w-0 space-y-16">
        <section id="connect" className="scroll-mt-32">
          <h2 className="font-display text-2xl">Make yourself at home.</h2>
          <p className="mt-3 mb-7 text-stone-500">
            Choose your client for its connection settings. Hive uses Streamable
            HTTP at https://hiveway.itssingularity.com/mcp and your Hive API
            key.
          </p>
          <ConnectionGuide />
        </section>
        <section id="credentials" className="scroll-mt-32">
          <h2 className="font-display text-2xl">
            One API key. A scoped connection.
          </h2>
          <p className="mt-4 leading-7 text-stone-600">
            Open{" "}
            <Link
              href="/dashboard?view=settings&tab=api-keys"
              className="underline"
            >
              Account settings → API keys
            </Link>{" "}
            and sign in with GitHub to create a named, short-lived key for free
            capabilities and free publishing. Connect a wallet for paid
            invocations or paid publishing. Keep the token private and use your
            client’s environment-variable support where available.
          </p>
          <p className="mt-3 leading-7 text-stone-600">
            Reading requires <code>capabilities:read</code>. Invoking hosted
            Wasm requires <code>capabilities:invoke</code>. Publishing requires{" "}
            <code>capabilities:publish</code>; workflows use{" "}
            <code>workflows:publish</code>. Expiry, revocation, per-call limits
            and total budgets are enforced by Hiveway.
          </p>
        </section>
        <section id="workflow" className="scroll-mt-32">
          <h2 className="font-display text-2xl">
            Give useful work a second life.
          </h2>
          <ol className="mt-5 list-decimal space-y-4 pl-5 leading-7 text-stone-600">
            <li>
              Ask your agent to use <code>search_capabilities</code> for the
              task at hand.
            </li>
            <li>
              Inspect the input schema, runtime and atomic-unit price before
              invoking the selected <code>hive_…</code> tool with a fresh UUID{" "}
              <code>idempotency_key</code>.
            </li>
            <li>
              Free tools execute directly. Paid tools return a quote and x402
              requirements. Your wallet client signs that specific payment;
              never send a private key.
            </li>
            <li>
              Read the receipt. Retry an uncertain call with the same key and
              input. A new key creates a new invocation.
            </li>
          </ol>
          <p className="mt-5 rounded-2xl bg-lime-50 p-5 text-sm leading-6 text-lime-950">
            Try this: “Search Hive for a capability that matches my task. Show
            me its price and schema, then invoke it with my approval. Keep its
            invocation ID so I can inspect the result.”
          </p>
        </section>
        <section id="external-mcps" className="scroll-mt-32">
          <h2 className="font-display text-2xl">Bring your own MCP servers.</h2>
          <p className="mt-4 text-sm leading-7 text-[#77796f]">
            Connect external stdio or HTTP MCP servers through a private Hive
            connector. The connector runs on your computer and makes outbound
            HTTPS requests, so you do not need a public home-network endpoint.
            Hive routing is free and requires no wallet; upstream services may
            charge separately.
          </p>
          <p className="mt-4 text-sm leading-7 text-[#77796f]">
            The official filesystem MCP is included. Configure explicit allowed
            directories and keep your Hive API key outside them. Your agent
            discovers namespaced tools through its existing Hive connection.
            Connections are private to your account and do not become public
            artifacts.
          </p>
          <a
            className="mt-4 inline-flex text-sm font-semibold text-[#52664b] underline underline-offset-4"
            href="https://github.com/parazeeknova/hive/tree/main/packages/connector"
          >
            Connector setup and filesystem configuration
          </a>
        </section>
        <section id="scaffold" className="scroll-mt-32">
          <h2 className="font-display text-2xl">
            Start from any public GitHub starter.
          </h2>
          <p className="mt-4 leading-7 text-stone-600">
            Ask your connected agent to call <code>scaffold_project</code> with
            <code> repository_url</code>, a full 40-character commit
            <code> revision</code>, and <code>project_name</code>. The source is
            your choice: Hive has no built-in repository or template snapshot.
            This free tool verifies a bounded public source archive and returns
            its download URL, checksum, and local setup instructions.
          </p>
          <p className="mt-3 leading-7 text-stone-600">
            Publish a free capability with runtime repository_scaffold to list
            this operation in Discover and invoke it with normal quotes and
            receipts. Its input contains project_name, repository_url and
            revision. The scaffold_project MCP name remains available for
            existing clients. Your agent downloads and verifies the archive,
            then uses its local filesystem MCP or shell tools to create a new
            directory. Existing directories are preserved, and source package
            names stay unchanged. Read the source README for its prerequisites
            and installation steps. Source scripts and dependency installation
            are never executed by Hive. Archives are limited to 8 MiB
            compressed, 32 MiB expanded and 2,000 files. Links and unsafe paths
            are rejected by the local installer.
          </p>
          <p className="mt-3 leading-7 text-stone-600">
            From a Hive checkout, run{" "}
            <code>
              bun scripts/scaffold.ts REPOSITORY_URL COMMIT_SHA my-app
            </code>{" "}
            with <code>HIVE_API_KEY</code> set. The installer verifies and
            writes the files and initializes local Git. Creating and pushing a
            GitHub repository uses your local GitHub CLI or GitHub MCP
            authorization separately. Never send the Hive API key to the source
            archive host.
          </p>
        </section>
        <section id="build" className="scroll-mt-32">
          <h2 className="font-display text-2xl">Ask Hive to build a tool.</h2>
          <p className="mt-4 leading-7 text-stone-600">
            Open Account settings → Capabilities → Build with Hive, or call{" "}
            <code>build_capability</code> over MCP with a new UUID{" "}
            <code>idempotency_key</code>, a name, description, task and one to
            eight examples containing <code>input</code> and{" "}
            <code>expected</code>. Poll <code>get_build</code> with the returned{" "}
            <code>build_id</code>. Hive generates Wasm code, compiles it and
            runs every supplied example. When they pass, it publishes a free
            artifact under your account. No wallet needed.
          </p>
          <p className="mt-3 leading-7 text-stone-600">
            These tools process JSON in an isolated runtime without filesystem
            or network access. For example, build a validator, formatter or
            calculator. Review generated source and test more inputs: passing
            examples is not a correctness guarantee. Builds are asynchronous,
            limited per account and across the server, and may fail without
            publishing. Reuse the same idempotency key and request when retrying
            a submission.
          </p>
          <p className="mt-3 leading-7 text-stone-600">
            Tasks, examples and generated code go to the server&apos;s
            configured model provider. Meta Contributor mode may use this
            content for training. Model and account limit appear in the
            dashboard. Your Hive API key and the server&apos;s model key are
            never included in generation prompts.
          </p>
        </section>
        <section id="publish" className="scroll-mt-32">
          <h2 className="font-display text-2xl">
            Publish something worth reusing.
          </h2>
          <p className="mt-4 leading-7 text-stone-600">
            Use <code>upload_artifact</code> to upload a JSON-in/JSON-out Wasm
            module and its input/output schemas. Then use{" "}
            <code>publish_capability</code> with its artifact ID, runtime{" "}
            <code>wasm</code>, and an atomic price string. Set{" "}
            <code>price: &quot;0&quot;</code> for a free capability. USDC uses
            six decimal places: <code>&quot;10000&quot;</code> is 0.01 USDC.
          </p>
          <p className="mt-3 leading-7 text-stone-600">
            Code and prices are immutable. Publish a new capability to change
            either. You can pause or restore your listing in Account settings;
            previous quotes and receipts stay pinned.
          </p>
        </section>
        <section id="boundaries" className="scroll-mt-32">
          <h2 className="font-display text-2xl">
            Clear boundaries. Useful tools.
          </h2>
          <div className="mt-5 grid gap-5 sm:grid-cols-2">
            <div className="rounded-3xl border border-stone-200 p-6">
              <h3 className="font-display font-semibold">
                Your agent stays in charge
              </h3>
              <p className="mt-3 text-sm leading-6 text-stone-500">
                Call Hive during your own workflow. Hosted Wasm has no
                filesystem or network access. It cannot call another hosted API
                from inside the sandbox.
              </p>
            </div>
            <div className="rounded-3xl border border-stone-200 p-6">
              <h3 className="font-display font-semibold">
                A receipt for each call
              </h3>
              <p className="mt-3 text-sm leading-6 text-stone-500">
                x402 payments settle to the creator. Dodo currently supports
                test checkout and merchant payables. Session budget and earnings
                history are not a spendable wallet balance.
              </p>
            </div>
          </div>
          <p className="mt-5 text-sm leading-6 text-stone-500">
            Client configuration examples follow the official documentation
            linked below each example. OpenCode has been tested live with Hive;
            the other examples are documentation-verified, not claims of
            end-to-end testing in every client.
          </p>
        </section>
      </div>
    </div>
  </div>
);
export default Docs;
