import Image from "next/image";
import Link from "next/link";

import { Icon } from "../../components/icon";
import CreatorSection from "./creator-section";
import EarningsSection from "./earnings-section";
import ExecutionComparison from "./execution-comparison";
import Hero from "./hero";
import NetworkSection from "./network-section";

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
        <span className={overline}>A few things, upfront</span>
        <h2 className={sectionHeading}>
          Good tools.
          <br />
          <span className="text-stone-400">Clear terms.</span>
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
            What does Hive actually run?
            <Icon name="plus" size={18} className={faqIcon} />
          </summary>
          <p className={faqBody}>
            The published implementation runs with the caller’s inputs. The
            result is generated for that request, so the same tool can serve
            different repositories, documents, or datasets.
          </p>
        </details>
        <details className={faqItem}>
          <summary className={faqSummary}>
            How does pricing work?
            <Icon name="plus" size={18} className={faqIcon} />
          </summary>
          <p className={faqBody}>
            Creators publish a fixed per-call price, including free
            capabilities. Hiveway enforces session budgets and records
            invocations and creator sales. Paid calls require a separate
            client-wallet authorization; the website provides the quote handoff
            to your agent.
          </p>
        </details>
        <details className={faqItem}>
          <summary className={faqSummary}>
            What does “verified” mean?
            <Icon name="plus" size={18} className={faqIcon} />
          </summary>
          <p className={faqBody}>
            A capability has passed a defined set of contract checks. Its
            listing should show the test evidence and version. Those checks are
            evidence you can inspect, rather than a promise that every result
            will be correct.
          </p>
        </details>
        <details className={faqItem}>
          <summary className={faqSummary}>
            What can I try today?
            <Icon name="plus" size={18} className={faqIcon} />
          </summary>
          <p className={faqBody}>
            Connect a scoped wallet session to search live capabilities, run
            free hosted Wasm with JSON input, inspect receipts, and manage
            listing availability. Agents can publish artifacts through MCP. Paid
            calls use quotes and explicit wallet authorization. Disconnected
            examples are illustrative.
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
