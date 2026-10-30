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
              <h3 className="font-semibold">Your agent stays in charge</h3>
              <p className="mt-3 text-sm leading-6 text-stone-500">
                Call Hive during your own workflow. Hosted Wasm has no
                filesystem or network access. It cannot call another hosted API
                from inside the sandbox.
              </p>
            </div>
            <div className="rounded-3xl border border-stone-200 p-6">
              <h3 className="font-semibold">A receipt for each call</h3>
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
