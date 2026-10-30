import Image from "next/image";
import Link from "next/link";

import { Icon } from "../../components/icon";
import { publicMetadata, websiteSchema } from "../../lib/seo";
import CreatorSection from "./creator-section";
import EarningsSection from "./earnings-section";
import ExecutionComparison from "./execution-comparison";
import Hero from "./hero";
import NetworkSection from "./network-section";

export const metadata = publicMetadata(
  "Hive — AI agent tools and MCP capabilities",
  "/"
);

const section =
  "mx-auto w-[calc(100%-4rem)] max-w-6xl max-[760px]:w-[calc(100%-2rem)]";
const overline = "text-xs font-bold tracking-widest text-stone-400 uppercase";
const sectionHeading =
  "mt-4 text-3xl leading-snug font-bold tracking-tight max-[760px]:text-2xl";
const faqItem =
  "group border-b border-stone-200 py-5 first:pt-0 [&_summary::-webkit-details-marker]:hidden";
const faqSummary =
  "flex list-none items-center justify-between gap-4 text-sm font-semibold";
const faqIcon =
  "text-stone-400 transition-transform duration-200 group-open:rotate-45";
const faqBody = "mt-3.5 pr-8 text-sm leading-loose text-stone-500";

const Home = () => (
  <>
    <script
      type="application/ld+json"
      // JSON-LD is generated from server-owned values and escapes HTML delimiters.
      // oxlint-disable-next-line react/no-danger
      dangerouslySetInnerHTML={{
        __html: JSON.stringify(websiteSchema).replaceAll("<", "\\u003C"),
      }}
    />
    <Hero />
    <NetworkSection />
    <ExecutionComparison />
    <EarningsSection />
    <CreatorSection />
    <section
      className={`${section} grid grid-cols-[1fr_1.4fr] gap-20 pb-20 max-[1000px]:grid-cols-1 max-[1000px]:gap-10`}
      id="docs"
    >
      <div>
        <span className={overline}>Getting started</span>
        <h2 className={sectionHeading}>
          A few questions,
          <br />
          <span className="text-stone-400">answered.</span>
        </h2>
        <Link
          href="/dashboard"
          className="mt-6 inline-flex items-center gap-2.5 text-xs font-medium text-neutral-700"
        >
          Explore the workspace <Icon name="arrow" size={16} />
        </Link>
      </div>
      <div>
        <details open className={faqItem}>
          <summary className={faqSummary}>
            How do I get started?
            <Icon name="plus" size={18} className={faqIcon} />
          </summary>
          <p className={faqBody}>
            Sign in with GitHub, find a capability in Discover, and try it with
            your own input. You can also connect your agent to Hive using an API
            key and MCP. The{" "}
            <Link href="/docs" className="underline underline-offset-4">
              setup guides
            </Link>{" "}
            walk you through it.
          </p>
        </details>
        <details className={faqItem}>
          <summary className={faqSummary}>
            Do I need a wallet to use Hive?
            <Icon name="plus" size={18} className={faqIcon} />
          </summary>
          <p className={faqBody}>
            No. Your GitHub account is enough to run free capabilities and
            create API keys for your agents. Connect a wallet when you want to
            use paid capabilities or publish paid tools.
          </p>
        </details>
        <details className={faqItem}>
          <summary className={faqSummary}>
            How much does a capability cost?
            <Icon name="plus" size={18} className={faqIcon} />
          </summary>
          <p className={faqBody}>
            Each listing shows its price per run. Free capabilities cost
            nothing. For a paid capability, review the quote and authorize
            payment before running it. Your agent’s spending limits help you
            control what it can spend.
          </p>
        </details>
        <details className={faqItem}>
          <summary className={faqSummary}>
            Can I publish a capability I built?
            <Icon name="plus" size={18} className={faqIcon} />
          </summary>
          <p className={faqBody}>
            Yes. Sign in with GitHub, upload your Wasm artifact, and add a name,
            description, input and output schemas, and price. Publish it from
            Account settings or through your agent. You can pause or restore the
            listing whenever you need to.
          </p>
        </details>
        <details className={faqItem}>
          <summary className={faqSummary}>
            Where can I find my results?
            <Icon name="plus" size={18} className={faqIcon} />
          </summary>
          <p className={faqBody}>
            Open Activity in your dashboard to see invocation status, receipts,
            and results for your current credential. Sample-test results appear
            in the test dialog. If a payment is still pending, check its receipt
            before starting another paid run.
          </p>
        </details>
      </div>
    </section>
    <section
      className={`${section} pb-8 text-center sm:pb-12`}
      aria-labelledby="closing-heading"
    >
      <div className="surface-light relative isolate overflow-hidden rounded-[2rem] px-6 py-14 sm:rounded-[2.5rem] sm:py-16">
        <Image
          src="/hive-footer.png"
          alt=""
          fill
          priority={false}
          sizes="(min-width: 1152px) 1152px, 100vw"
          className="-z-10 object-cover"
        />
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 -z-10 bg-linear-to-b from-white/95 via-white/60 to-transparent"
        />
        <span className={overline}>There’s a tool for that.</span>
        <h2
          id="closing-heading"
          className="mt-5 mb-7 text-3xl leading-snug font-semibold tracking-tight max-[760px]:text-2xl"
        >
          And a hive for what’s next.
        </h2>
        <Link
          href="/dashboard"
          className="surface-dark inline-flex min-h-12 items-center justify-center gap-3 rounded-full px-6 py-3 text-sm font-medium text-white transition-transform duration-200 hover:-translate-y-0.5 active:translate-y-0 motion-reduce:transform-none"
        >
          Come on in <Icon name="arrow" size={18} />
        </Link>
      </div>
    </section>
  </>
);

export default Home;
