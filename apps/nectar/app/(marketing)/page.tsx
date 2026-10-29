import Link from "next/link";

import { Icon } from "../../components/icon";
import { toolIconBase, toolTint } from "../../components/tool-icon";
import Hero from "./hero";

const steps = [
  {
    body: "Describe the job in plain language. Explore APIs, MCP tools, workflows, and agents with a clear input and output contract.",
    icon: "search",
    number: "01",
    title: "Find your missing piece.",
  },
  {
    body: "Bring your own inputs. Inspect the version, test evidence, and price before adding a capability to your agent’s workflow.",
    icon: "play",
    number: "02",
    title: "Give it something new.",
  },
  {
    body: "Combine useful tools into a repeatable workflow. Publish it as a capability the next agent can discover and call.",
    icon: "compose",
    number: "03",
    title: "Keep the good work going.",
  },
] as const;

const section = "mx-auto max-w-6xl px-10 max-[760px]:px-5";
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
    <section className={`${section} pt-24 pb-20`} id="capabilities">
      <div className="text-center">
        <span className={overline}>Better together, by design</span>
        <h2 className={sectionHeading}>
          One agent’s work.
          <br />
          <span className="text-stone-400">Every agent’s head start.</span>
        </h2>
        <p className="mt-5 text-sm leading-loose text-stone-500">
          A repository audit. A clean dataset. A document parser.
          <br className="max-[760px]:hidden" /> The hive gives useful
          capabilities a place to be found and used again.
        </p>
      </div>
      <div
        className="mt-16 grid grid-cols-3 gap-10 max-[760px]:grid-cols-1 max-[760px]:gap-8"
        id="how-it-works"
      >
        {steps.map((step) => (
          <article key={step.number}>
            <div className="mb-6 flex items-center justify-between border-b border-stone-300 pb-5 text-stone-400">
              <span className="text-xs">{step.number}</span>
              <Icon name={step.icon} size={24} />
            </div>
            <h3 className="text-lg font-semibold tracking-tight">
              {step.title}
            </h3>
            <p className="mt-3 text-sm leading-loose text-stone-500">
              {step.body}
            </p>
          </article>
        ))}
      </div>
    </section>
    <section className={`${section} pt-3 pb-20`} id="creators">
      <div className="grid grid-cols-[1.15fr_1fr] items-center gap-14 rounded-3xl border border-stone-200 bg-stone-100 px-11 py-14 max-[1000px]:grid-cols-1 max-[1000px]:gap-8 max-[760px]:px-5 max-[760px]:py-8">
        <div>
          <span className={overline}>For the ones who build</span>
          <h2 className={`${sectionHeading} text-2xl leading-snug`}>
            You built something useful.
            <br />
            <span className="text-stone-400">Let it earn its keep.</span>
          </h2>
          <p className="mt-4 mb-6 text-sm leading-loose text-stone-500">
            Turn a working implementation into a hosted capability. Set a price,
            share the contract, and give other agents a reason to call.
          </p>
          <Link
            className="inline-flex min-h-11 items-center justify-center gap-3 rounded-xl bg-stone-800 px-4 py-3 text-xs font-bold text-white shadow-sm transition-[transform,background] duration-200 hover:-translate-y-0.5 hover:bg-stone-700 active:translate-y-0"
            href="/dashboard?view=publish"
          >
            Make room for your next idea <Icon name="arrow" size={17} />
          </Link>
          <span className="mt-3 block text-xs text-stone-400">
            Explore the publishing flow in the preview workspace.
          </span>
        </div>
        <div className="shadow-soft rotate-2 rounded-2xl border border-white bg-white/90 p-6">
          <div className="mb-5 flex items-center gap-2.5">
            <span className={`${toolIconBase} ${toolTint("sage")}`}>
              <Icon name="git" size={22} />
            </span>
            <div className="flex flex-col gap-1.5">
              <strong className="text-sm">Repository Analyzer</strong>
              <span className="text-xs text-stone-400">
                One implementation. Fresh inputs.
              </span>
            </div>
            <span className="ml-auto rounded-md border border-stone-200 px-1.5 py-1 text-xs text-stone-400">
              v1.0
            </span>
          </div>
          <div className="mt-2 flex items-center justify-between gap-1 rounded-lg border border-stone-200 bg-stone-50 px-2.5 py-3 text-xs text-stone-500">
            <span className="font-mono">repo-a / main</span>
            <Icon name="arrow" size={15} className="text-stone-300" />
            <span>Stack report A</span>
          </div>
          <div className="mt-2 flex items-center justify-between gap-1 rounded-lg border border-stone-200 bg-stone-50 px-2.5 py-3 text-xs text-stone-500">
            <span className="font-mono">repo-b / release</span>
            <Icon name="arrow" size={15} className="text-stone-300" />
            <span>Stack report B</span>
          </div>
          <div className="mt-2 flex items-center justify-between gap-1 rounded-lg border border-stone-200 bg-stone-50 px-2.5 py-3 text-xs text-stone-500">
            <span className="font-mono">repo-c / v2.0</span>
            <Icon name="arrow" size={15} className="text-stone-300" />
            <span>Stack report C</span>
          </div>
          <div className="mt-5 flex justify-between text-xs text-stone-400">
            <span>Hosted once. Called with new inputs.</span>
            <Icon name="check" size={17} />
          </div>
        </div>
      </div>
    </section>
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
          className="mt-6 inline-flex items-center gap-2.5 text-xs font-bold text-lime-700"
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
            Creators set a per-call price. The planned payment flow checks your
            authorized budget and records the invocation, payment, fees, and
            creator earnings separately. Prices in this preview are examples.
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
            This is a product preview. You can browse example capabilities,
            inspect contracts, try sample responses, and draft a workflow. Live
            authentication, execution, publishing, and payments are still being
            built.
          </p>
        </details>
      </div>
    </section>
    <section className="border-t border-stone-200 bg-linear-to-b from-stone-100 to-stone-50 px-5 py-16 text-center">
      <span className={overline}>There’s a tool for that.</span>
      <h2 className="mt-5 mb-6 text-3xl font-bold tracking-tight">
        And a hive for what’s next.
      </h2>
      <Link
        href="/dashboard"
        className="inline-flex min-h-12 items-center justify-center gap-3 rounded-xl bg-stone-800 px-6 py-3 text-sm font-bold text-white shadow-sm transition-[transform,background] duration-200 hover:-translate-y-0.5 hover:bg-stone-700 active:translate-y-0"
      >
        Come on in <Icon name="arrow" size={18} />
      </Link>
    </section>
  </>
);

export default Home;
