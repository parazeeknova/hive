import Image from "next/image";
import Link from "next/link";

import { Icon } from "../../components/icon";
import { JsonLd } from "../../components/json-ld";
import {
  applicationSchema,
  faqSchema,
  organizationSchema,
  publicMetadata,
  websiteSchema,
} from "../../lib/seo";
import HeroSearch from "./hero-search";
import HeroVideo from "./hero-video";
import { BenchmarkChart, benchmarks } from "./hive-benchmarks";
import type { Benchmark } from "./hive-benchmarks";
import { AgentConnect, EarningsPreview } from "./landing-interactive";
import LandingMotion from "./landing-motion";
import { LandingButton } from "./landing-shell";

export const metadata = publicMetadata(
  "Hive — Good work deserves a second life.",
  "/",
  "A shared toolbox for AI agents. Discover reusable capabilities, connect through MCP, and publish tools that earn on every call."
);
const network = [
  {
    body: "Before your agent builds from scratch, let it search the hive. Find reusable capabilities with clear inputs, outputs, and a price you can see up front.",
    label: "Discover",
    surface: "",
    title: "A head start, already built.",
  },
  {
    body: "Inspect the contract, try a sample, and run a capability on your own task. Bring a working tool into your workflow without rebuilding its implementation.",
    label: "Invoke",
    surface: "-2",
    title: "Their work. Your inputs.",
  },
  {
    body: "Connect compatible capabilities into a workflow. Turn individual steps into a reusable contribution, so the next agent starts further along.",
    label: "Compose",
    surface: "-3",
    title: "Small tools. Bigger possibilities.",
  },
  {
    body: "Share a capability for free or set a price per invocation. Give other agents something worth calling, and earn when they use your paid tool.",
    label: "Publish",
    surface: "-4",
    title: "Built once. Useful again.",
  },
];
const workflow = [
  {
    alt: "A honeybee exploring white chamomile flowers in a quiet meadow",
    description:
      "Describe the job before you build the solution. Search Hive for a capability that already does the work, then decide whether it fits.",
    details: [
      "Inspect the publisher and input schema.",
      "Check the output contract and price per run.",
      "Try a sample before using your own inputs.",
    ],
    href: "/dashboard?view=discover",
    image: "/landing/workflow-find.webp",
    link: "Explore capabilities",
    summary: "Search by the task. Inspect the schema, publisher, and price.",
    title: "Find",
  },
  {
    alt: "Two honeybees working together on a pale meadow flower",
    description:
      "Bring the shared toolbox into the place you already work. Add Hive to your coding agent through a single authenticated MCP connection.",
    details: [
      "Create a scoped API key in your account.",
      "Choose your agent and add the MCP endpoint.",
      "Keep your existing model and workflow.",
    ],
    href: "#showcase-agent",
    image: "/landing/workflow-connect.webp",
    link: "Connect your agent",
    summary: "Add Hive to your agent with MCP and a scoped API key.",
    title: "Connect",
  },
  {
    alt: "A honeybee in flight above a white daisy",
    description:
      "Give a capability your inputs and put its implementation to work. Get the result back in your session, with a record you can follow.",
    details: [
      "Review and authorize a paid quote first.",
      "Invoke the tool with its expected inputs.",
      "Find status, results, and receipts in Activity.",
    ],
    href: "/dashboard?view=activity",
    image: "/landing/workflow-run.webp",
    link: "Open your workspace",
    summary: "Send your inputs. Get the output, status, and receipt.",
    title: "Run",
  },
  {
    alt: "A honeybee carrying golden pollen on a flowering clover",
    description:
      "Turn something useful into a tool other agents can call. Define its contract, choose your terms, and give the next agent a head start.",
    details: [
      "Upload your artifact and describe what it does.",
      "Share it for free or set a price per call.",
      "Pause or restore the listing when you need to.",
    ],
    href: "/dashboard?view=settings",
    image: "/landing/workflow-share.webp",
    link: "Publish a capability",
    summary: "Publish what works. Let the next agent build on it.",
    title: "Share",
  },
] as const;

const showcases = [
  {
    alt: "Natural honeycomb cells built together by bees",
    description:
      "Bring templr’s design library, Context7’s documentation, Tinyfish’s research, and your own MCP tools behind one discoverable Hive connection.",
    details: [
      {
        body: "Search templr for a relevant design and inspect its real previews before choosing a starting point.",
        title: "Find a direction",
      },
      {
        body: "Connect your own Context7 or Tinyfish account for documentation and research. Keep upstream credentials with the connector.",
        title: "Add the context",
      },
      {
        body: "Use the same discovery flow for a custom stdio or HTTP MCP. Your agent can find the right tool as the task changes.",
        title: "Bring your own tools",
      },
    ],
    facts: [
      ["Discover", "Tools for the task"],
      ["Connect", "Your MCP accounts"],
      ["Reuse", "One Hive endpoint"],
    ],
    flow: [
      { icon: "layers", label: "templr" },
      { icon: "book", label: "Context7" },
      { icon: "globe", label: "Tinyfish" },
    ],
    image: "/landing/creator-build.webp",
    kind: "network",
    note: "Private connections stay with your account. Upstream usage and billing still apply.",
    subtitle: "01 / Your tools, brought together",
    title: "A whole workflow. One hive.",
  },
  {
    alt: "Two honeybees working on the same flower",
    description:
      "Connect Hive once in Claude Code, Codex, or OpenCode. Your agent can search for relevant tools and call them without rebuilding their implementations.",
    details: [],
    facts: [
      ["Choose", "Your agent"],
      ["Authenticate", "A scoped API key"],
      ["Work", "Discover, then call"],
    ],
    flow: [
      { icon: "code", label: "Your agent" },
      { icon: "layers", label: "Hive MCP" },
      { icon: "bolt", label: "The right tool" },
    ],
    image: "/landing/workflow-connect.webp",
    kind: "agent",
    note: "Create a key in Account settings and set HIVE_API_KEY before running the command.",
    subtitle: "02 / Bring Hive into your agent",
    title: "One connection. More reach.",
  },
  {
    alt: "A drop of honey collected from a natural honeycomb",
    description:
      "A paid capability comes with a quote for that invocation. With an x402-capable wallet, your agent can authorize the payment and continue to the result within your spending rules.",
    details: [
      {
        body: "Hive returns the price and payment requirements for the capability and inputs your agent selected.",
        title: "Quote the work",
      },
      {
        body: "Your client wallet signs the authorized payment. Scoped credentials and spending limits keep access and cost bounded.",
        title: "Let your agent pay",
      },
      {
        body: "The capability runs after payment verification. Follow its invocation status, result, and receipt in Activity.",
        title: "Get the result and receipt",
      },
    ],
    facts: [
      ["Price", "Per invocation"],
      ["Authorize", "Your client wallet"],
      ["Track", "Status & receipts"],
    ],
    flow: [
      { icon: "file", label: "Quote" },
      { icon: "wallet", label: "Authorize" },
      { icon: "check", label: "Execute" },
    ],
    image: "/landing/creator-earn.webp",
    kind: "payment",
    note: "Automatic payment requires client wallet support. Hosted checkout is a separate manual flow; free capabilities need no payment.",
    subtitle: "03 / Agent-native payments",
    title: "Pay for the call. Keep moving.",
  },
] as const;

const practicalUses = [
  {
    body: "Compile a useful JSON-in, JSON-out tool to Wasm, publish its immutable artifact, and let other agents invoke the same implementation. A shared starting point without rebuilding the tool for every session.",
    label: "REUSABLE ARTIFACTS",
    note: "Hive reuses published executable artifacts. It does not cache arbitrary project build outputs.",
    steps: ["Compile once", "Publish the artifact", "Invoke through MCP"],
    thumb: "/landing/pill-tools.webp",
    title: "Build once. Call it again.",
  },
  {
    body: "Hook up a filesystem MCP beside your project. Let your agent inspect source, read context, and write files through the same Hive connection, with access limited to directories you approve locally.",
    label: "FILESYSTEM ACCESS",
    note: "Run the connector on your computer to access local files. Restarting clears session directory approvals.",
    steps: [
      "Approve a directory",
      "Read or write files",
      "Stay in your workflow",
    ],
    thumb: "/landing/pill-thinking.webp",
    title: "Your workspace, connected.",
  },
  {
    body: "Compose up to four connected MCP capabilities into a reusable sequence. For example: inspect approved files, look up documentation, and write notes using tools connected to your own account.",
    label: "CONNECTED WORKFLOWS",
    note: "Supply each step’s inputs explicitly. Connected workflows run sequentially, return ordered results, and are free on Hive.",
    steps: ["Read the source", "Look up the docs", "Save the findings"],
    thumb: "/landing/pill-next.webp",
    title: "Make the steps work together.",
  },
] as const;

const chapters = [
  {
    alt: "Fresh beeswax cells being built by a honeybee",
    id: "build",
    image: "/landing/creator-build.webp",
    items: [
      "Start with a working implementation.",
      "Give it explicit input and output schemas.",
      "Test the tool before sharing it.",
    ],
    name: "Build.",
    subtitle: "01 / Make something worth calling",
  },
  {
    alt: "Honeybees arriving at a wooden hive entrance",
    id: "publish",
    image: "/landing/creator-publish.webp",
    items: [
      "Upload your Wasm artifact from Account settings.",
      "Add a clear name, description, and price.",
      "Keep control: pause or restore your listing.",
    ],
    name: "Publish.",
    subtitle: "02 / Put your work on the map",
  },
  {
    alt: "A drop of honey on the edge of a natural honeycomb",
    id: "earn",
    image: "/landing/creator-earn.webp",
    items: [
      "Share for free or charge per invocation.",
      "Earn when agents run your paid capability.",
      "See your sales and earnings in your account.",
    ],
    name: "Earn.",
    subtitle: "03 / Make every call count",
  },
  {
    alt: "A flourishing meadow of chamomile and clover",
    id: "grow",
    image: "/landing/creator-repeat.webp",
    items: [
      "Watch how your capabilities are used.",
      "Turn another useful idea into a callable tool.",
      "Make the shared toolbox a little better.",
    ],
    name: "Repeat.",
    subtitle: "04 / Give the next agent a head start",
  },
];
const safeguards = [
  { benchmark: benchmarks[0], image: "/bee-right.png", title: "Done sooner." },
  {
    benchmark: benchmarks[1],
    image: "/coin.png",
    title: "Fewer tokens spent.",
  },
  { benchmark: benchmarks[2], image: "/bee-left.png", title: "Less per task." },
].filter((item): item is typeof item & { benchmark: Benchmark } =>
  Boolean(item.benchmark)
);
const faqs = [
  {
    answer: (
      <>
        Sign in with GitHub, open Discover, and try a capability with your own
        input. To use Hive inside your agent, create an API key and follow the{" "}
        <Link className="hive-link" href="/docs">
          MCP setup guide
        </Link>
        .
      </>
    ),
    question: "How do I get started?",
    // Plain-text twin of the linked answer, for FAQ structured data.
    text: "Sign in with GitHub, open Discover, and try a capability with your own input. To use Hive inside your agent, create an API key and follow the MCP setup guide.",
  },
  {
    answer:
      "GitHub is enough to run free capabilities and create API keys. Connect a wallet when you want to use paid capabilities or publish paid tools.",
    question: "Do I need a wallet?",
  },
  {
    answer:
      "Each listing shows its price per run. Free capabilities cost nothing. For a paid capability, review the quote and authorize payment before it runs. Your agent’s spending limits help you stay in control.",
    question: "What does a capability cost?",
  },
  {
    answer:
      "Yes, if your client supports authenticated HTTP MCP. Connect using a scoped API key. The setup guide includes instructions for Claude Code, Codex, and OpenCode.",
    question: "Can my agent use Hive?",
  },
  {
    answer:
      "Yes. Sign in with GitHub, upload your Wasm artifact, and add a name, description, input and output schemas, and price. Publish from Account settings or through your agent, and pause or restore the listing whenever you need to.",
    question: "Can I publish something I built?",
  },
  {
    answer:
      "Open Activity in your dashboard for invocation status, receipts, and results for your current credential. Sample-test results appear in the test dialog.",
    question: "Where do my results go?",
  },
];
const SectionHeading = ({
  id,
  title,
  children,
}: {
  id: string;
  title: string;
  children: React.ReactNode;
}) => (
  <div className="hive-section-head">
    <h2 className="hive-text-3" id={id} data-hive-rise>
      {title}
    </h2>
    <p className="hive-text-5">{children}</p>
  </div>
);
const Home = () => (
  <>
    <JsonLd
      data={[
        { ...websiteSchema, publisher: organizationSchema },
        organizationSchema,
        applicationSchema,
        faqSchema(
          faqs.map((item) => ({
            answer: item.text ?? String(item.answer),
            question: item.question,
          }))
        ),
      ]}
    />
    <script
      // Start every landing visit at the top so the hero intro plays. Anchor
      // visits keep their hash so in-page links still land correctly.
      // oxlint-disable-next-line react/no-danger -- Static inline script with no interpolated values.
      dangerouslySetInnerHTML={{
        __html: `try{history.scrollRestoration="manual"}catch(e){}if(!location.hash)window.scrollTo(0,0)`,
      }}
    />
    <LandingMotion />
    <section id="welcome" aria-labelledby="welcome-title">
      <div className="hero__plot">
        <div className="hero__photo hive-plot">
          <div className="hero__sticky">
            <div className="hero__pan">
              <HeroVideo className="hero__world" />
            </div>
            <p className="hero__scene-label">
              A little less reinventing. A little more making.
            </p>
          </div>
        </div>
        <div className="hero__second">
          <div className="hero__panel hive-surface">
            <div>
              <p className="hero__eyebrow hive-text-4">
                BUILT BY AGENTS. USED BY AGENTS.
              </p>
              <h1 className="hive-text-3 hero__statement" id="welcome-title">
                <span className="hero__sline">Good work</span>
                <span className="hero__sline">deserves</span>
                <span className="hero__sline">a second life.</span>
              </h1>
              <p className="hero__description">
                A shared toolbox for AI agents. Find a capability, put it to
                work, or publish your own for the next agent to use.
              </p>
            </div>
            <div className="hero__foot">
              <div className="hero__search">
                <HeroSearch />
              </div>
              <ul
                className="hero__never hive-text-4"
                aria-label="Ways to use Hive"
              >
                <li>Find a useful tool</li>
                <li>Connect your agent</li>
                <li>Share what you build</li>
              </ul>
              <div className="hero__cta">
                <LandingButton href="/dashboard">
                  Find your head start{" "}
                  <span className="hive-inline-arrow" aria-hidden="true" />
                </LandingButton>
                <a className="hive-link" href="#workflow">
                  How Hive works
                </a>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
    <section id="network" aria-labelledby="network-title">
      <SectionHeading id="network-title" title="Someone’s already solved that.">
        One agent’s useful work can become another agent’s starting point. Hive
        makes those connections possible.
      </SectionHeading>
      <ol className="network__bands">
        {network.map((item) => (
          <li className="network__band" key={item.label}>
            <div className="network__inner hive-plot__ground">
              <div
                className={`network__panel hive-plot hive-surface${item.surface}`}
              >
                <h3 className="hive-text-2 network__title">{item.title}</h3>
                <p className="hive-text network__text">{item.body}</p>
              </div>
              <span className="network__tab hive-plot__tab hive-plot__tab--tl hive-text-4">
                {item.label}
              </span>
            </div>
          </li>
        ))}
      </ol>
    </section>
    <section id="workflow" aria-labelledby="workflow-title">
      <SectionHeading id="workflow-title" title="From “how?” to done.">
        Discover, connect, run, share. A common language for tools, so your
        agent can spend more time on the work that’s yours.
      </SectionHeading>
      <div className="flow__stage">
        <div className="flow__board">
          {workflow.map((item, index) => (
            <article
              key={item.title}
              className={`flow__bed flow__bed--${index + 1}`}
            >
              <Image
                className="flow__img"
                src={item.image}
                alt={item.alt}
                width={1400}
                height={1000}
                sizes="(max-width: 700px) 100vw, 70vw"
              />
              <div className="flow__card">
                <h3 className="flow__name hive-text-6">
                  <span className="flow__step">0{index + 1}</span>
                  {item.title}
                </h3>
                <div className="flow__text hive-text">
                  <p>{item.summary}</p>
                  <div className="flow__detail">
                    <p>{item.description}</p>
                    <ul>
                      {item.details.map((detail) => (
                        <li key={detail}>{detail}</li>
                      ))}
                    </ul>
                    <Link className="hive-link" href={item.href}>
                      {item.link}
                      <span className="hive-inline-arrow" aria-hidden="true" />
                    </Link>
                  </div>
                </div>
              </div>
            </article>
          ))}
        </div>
      </div>
    </section>
    <section id="shared-work" aria-labelledby="shared-work-title">
      <div className="manifesto__plot">
        <h2
          className="manifesto__label hive-plot__tab hive-plot__tab--tl hive-text-6"
          id="shared-work-title"
        >
          A shared head start
        </h2>
        <div className="manifesto__panel hive-plot hive-surface-2">
          <p className="hive-text-7 manifesto__sentence">
            Your agent brings the thinking{" "}
            <span className="manifesto__pill">
              <Image
                src="/landing/pill-thinking.webp"
                alt=""
                width={180}
                height={100}
              />
            </span>
            . The hive brings the tools{" "}
            <span className="manifesto__pill">
              <Image
                src="/landing/pill-tools.webp"
                alt=""
                width={180}
                height={100}
              />
            </span>
            . Every useful capability{" "}
            <span className="manifesto__pill">
              <Image
                src="/landing/pill-capability.webp"
                alt=""
                width={180}
                height={100}
              />
            </span>{" "}
            gives the next idea a little more room to fly{" "}
            <span className="manifesto__pill">
              <Image
                src="/landing/pill-next.webp"
                alt=""
                width={180}
                height={100}
              />
            </span>
            .
          </p>
        </div>
        <div className="manifesto__foot hive-plot__tab hive-plot__tab--br">
          <p className="hive-text">
            Your model. Your workflow. A whole hive of possibilities.
          </p>
          <a className="hive-link" href="#connect">
            Explore what you can do
          </a>
        </div>
      </div>
    </section>
    <section id="capabilities" aria-labelledby="capabilities-title">
      <SectionHeading id="capabilities-title" title="Less starting from zero.">
        Bring your tools together, connect your agent, and pay for useful work
        as it happens. Three ways Hive takes a task further than a blank
        session.
      </SectionHeading>
      <div className="showcase__list">
        {showcases.map((item) => (
          <article
            className="showcase__item"
            id={`showcase-${item.kind}`}
            key={item.title}
          >
            <div className="showcase__inner hive-plot__ground">
              <div className="showcase__frame hive-plot">
                <Image
                  className="showcase__img"
                  src={item.image}
                  alt={item.alt}
                  width={1600}
                  height={1000}
                  sizes="100vw"
                />
                <div className="showcase__shade" />
                <div className="showcase__content">
                  <p className="showcase__description">{item.description}</p>
                  <div className="showcase__flow" aria-label="Example workflow">
                    {item.flow.map((step, index) => (
                      <span key={step.label}>
                        <Icon name={step.icon} size={18} />
                        {step.label}
                        {index < item.flow.length - 1 && (
                          <Icon
                            name="arrow"
                            size={14}
                            className="showcase__flow-arrow"
                          />
                        )}
                      </span>
                    ))}
                  </div>
                  {item.kind === "agent" ? (
                    <AgentConnect />
                  ) : (
                    <ol className="showcase__steps">
                      {item.details.map((step, index) => (
                        <li key={step.title}>
                          <span className="showcase__step-number">
                            0{index + 1}
                          </span>
                          <div>
                            <h4>{step.title}</h4>
                            <p>{step.body}</p>
                          </div>
                        </li>
                      ))}
                    </ol>
                  )}
                  {item.kind !== "agent" && (
                    <p className="showcase__note">{item.note}</p>
                  )}
                </div>
              </div>
              <div className="showcase__name hive-plot__tab hive-plot__tab--tl">
                <h3 className="hive-text-6">{item.title}</h3>
                <p className="hive-text-4">{item.subtitle}</p>
              </div>
              <dl className="showcase__facts hive-plot__tab hive-plot__tab--br">
                {item.facts.map(([label, value]) => (
                  <div key={label}>
                    <dt className="hive-text-4">{label}</dt>
                    <dd>{value}</dd>
                  </div>
                ))}
              </dl>
            </div>
          </article>
        ))}
      </div>
    </section>
    <section id="connect" aria-labelledby="connect-title">
      <SectionHeading id="connect-title" title="Put the connection to work.">
        Keep useful artifacts close, bring your files into reach, and turn
        repeatable steps into workflows. More of your agent’s work can carry
        forward.
      </SectionHeading>
      <div className="clients__list">
        {practicalUses.map((item, index) => (
          <article className="clients__row" key={item.title}>
            <div className="clients__thumb">
              <Image
                src={item.thumb}
                alt=""
                fill
                sizes="120px"
                loading="lazy"
              />
            </div>
            <h3 className="hive-text-6">{item.title}</h3>
            <div className="clients__detail">
              <p className="hive-text">{item.body}</p>
              <ol className="use-case__steps">
                {item.steps.map((step, stepIndex) => (
                  <li key={step}>
                    <span>0{stepIndex + 1}</span>
                    {step}
                  </li>
                ))}
              </ol>
              <p className="use-case__note">{item.note}</p>
            </div>
            <p className="clients__from hive-text-4">
              0{index + 1} · {item.label}
            </p>
          </article>
        ))}
      </div>
      <p className="clients__more">
        <Link className="hive-link" href="/docs">
          Explore the setup
          <span className="hive-inline-arrow" aria-hidden="true" />
        </Link>
      </p>
    </section>
    <section id="creators" aria-labelledby="creators-title">
      <SectionHeading
        id="creators-title"
        title="You built it. Let it go further."
      >
        Your next useful tool could be someone else’s missing piece. Publish a
        capability, choose your terms, and give your work a life beyond one
        session.
      </SectionHeading>
      <div className="chapters__pin">
        <div className="chapters__track">
          {chapters.map((item, index) => (
            <article
              className={`chapters__panel chapters__panel--${item.id}`}
              key={item.id}
            >
              <div className="chapters__photo hive-plot">
                <Image
                  src={item.image}
                  alt={item.alt}
                  fill
                  sizes="(max-width: 860px) 84vw, 32vw"
                />
              </div>
              <div className="chapters__body">
                <div>
                  <h3 className="hive-text-2 chapters__name">{item.name}</h3>
                  <p className="hive-text-4 chapters__months">
                    {item.subtitle}
                  </p>
                </div>
                {index === 2 ? (
                  <EarningsPreview />
                ) : (
                  <ul className="chapters__visits hive-text">
                    {item.items.map((text) => (
                      <li key={text}>{text}</li>
                    ))}
                  </ul>
                )}
                <Link
                  className="hive-link chapters__link"
                  href="/dashboard?view=settings"
                >
                  {index === 2 ? "Start earning" : "Build for the hive"}{" "}
                  <span className="hive-inline-arrow" aria-hidden="true" />
                </Link>
              </div>
            </article>
          ))}
        </div>
      </div>
    </section>
    <section id="control" aria-labelledby="control-title">
      <SectionHeading id="control-title" title="Less rebuilding. More done.">
        The same tasks, run with and without Hive on GPT-6.1 Sol and DeepSeek
        V4.1 Flash. Reusing a tool beats regenerating it.
      </SectionHeading>
      <ul className="assurance__list">
        {safeguards.map((item, index) => (
          <li className="assurance__card" key={item.title}>
            <div
              className={`assurance__panel assurance__panel--${index + 1} hive-plot`}
            >
              <h3 className="hive-text-6">{item.title}</h3>
              <BenchmarkChart data={item.benchmark} />
            </div>
            <div className="assurance__thumb">
              <Image src={item.image} alt="" width={240} height={160} />
            </div>
          </li>
        ))}
      </ul>
    </section>
    <section id="questions" aria-labelledby="questions-title">
      <div className="faq__head">
        <h2 className="hive-text-3" id="questions-title" data-hive-rise>
          A few things you might be wondering.
        </h2>
        <p className="hive-text">
          Start with your GitHub account. Bring your agent when you’re ready.
          The{" "}
          <Link className="hive-link" href="/docs">
            docs
          </Link>{" "}
          cover the details.
        </p>
      </div>
      <div className="faq__list">
        {faqs.map((item) => (
          <details className="faq__item" key={item.question}>
            <summary>
              <span className="hive-text-6">{item.question}</span>
              <span className="faq__mark" aria-hidden="true" />
            </summary>
            <p className="hive-text">{item.answer}</p>
          </details>
        ))}
      </div>
    </section>
    <section id="start" aria-labelledby="start-title">
      <div className="join__plot">
        <div className="join__panel hive-plot hive-surface">
          <div className="join__intro">
            <span className="hive-text-4">THERE’S A TOOL FOR THAT.</span>
            <h2 className="hive-text-2 join__title" id="start-title">
              And a hive for what’s next.
            </h2>
            <p className="hive-text-5">
              Come with a task. Leave with a head start. Find something useful,
              or make something the next agent will thank you for.
            </p>
            <LandingButton href="/dashboard">
              Come on in{" "}
              <span className="hive-inline-arrow" aria-hidden="true" />
            </LandingButton>
          </div>
          <div className="join__visual">
            <Image src="/bee-left.png" alt="" width={325} height={304} />
          </div>
          <form className="join__search" action="/dashboard" method="get">
            <input type="hidden" name="view" value="discover" />
            <label htmlFor="landing-search" className="hive-text-4">
              What are you working on?
            </label>
            <div>
              <input
                className="hive-field__input"
                id="landing-search"
                type="search"
                name="q"
                placeholder="Try “repository analysis”"
              />
              <button type="submit" className="hive-btn">
                <span className="hive-btn__label">
                  Find a capability{" "}
                  <span className="hive-inline-arrow" aria-hidden="true" />
                </span>
              </button>
            </div>
          </form>
        </div>
        <dl className="join__tab hive-plot__tab hive-plot__tab--tr">
          <div>
            <dt className="hive-text-4">For the curious</dt>
            <dd>
              <Link className="hive-link" href="/dashboard?view=discover">
                Explore the capabilities{" "}
                <span className="hive-inline-arrow" aria-hidden="true" />
              </Link>
            </dd>
          </div>
          <div>
            <dt className="hive-text-4">For the builders</dt>
            <dd>
              <Link className="hive-link" href="/dashboard?view=settings">
                Publish your first tool{" "}
                <span className="hive-inline-arrow" aria-hidden="true" />
              </Link>
            </dd>
          </div>
          <div>
            <dt className="hive-text-4">For your agent</dt>
            <dd>
              <Link className="hive-link" href="/docs">
                Read the setup guide{" "}
                <span className="hive-inline-arrow" aria-hidden="true" />
              </Link>
            </dd>
          </div>
        </dl>
      </div>
    </section>
  </>
);
export default Home;
