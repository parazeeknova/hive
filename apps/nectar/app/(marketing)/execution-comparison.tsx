/* oxlint-disable jsx-a11y/prefer-tag-over-role -- SVG charts require image semantics. */
"use client";
import { useState } from "react";

const scenarios = [
  {
    context: "Custom instructions, pinned starter, local validation",
    name: "A Next.js project, ready to work",
    short: "Project scaffolding",
    steps: [
      "Discover a suitable starter and pin its commit",
      "Verify the archive and prepare a fresh directory",
      "Let your agent apply the custom instructions",
      "Install dependencies and validate locally",
    ],
    withHive: 90,
    withoutHive: 1020,
  },
  {
    context: "Existing hosted repository analyzers",
    name: "Understand an unfamiliar repository",
    short: "Repository analysis",
    steps: [
      "Discover repository and dependency analyzers",
      "Pass the repository URL and full commit",
      "Invoke the published capabilities",
      "Bring structured results back to your agent",
    ],
    withHive: 60,
    withoutHive: 720,
  },
  {
    context: "Reuse an existing published pure JSON capability",
    name: "Validate the next batch of data",
    short: "Reusable validation",
    steps: [
      "Find a published validator matching the input schema",
      "Submit a JSON object with a fresh invocation key",
      "Execute the versioned tool inside Hive",
      "Return its structured result and receipt",
    ],
    withHive: 15,
    withoutHive: 300,
  },
];
const formatTime = (seconds: number) =>
  seconds >= 60 ? `${Number((seconds / 60).toFixed(1))} min` : `${seconds} sec`;
const width = (seconds: number) => Math.max(8, (seconds / 1200) * 550);
const ExecutionComparison = () => {
  const [index, setIndex] = useState(0);
  const scenario = scenarios[index] ?? scenarios[0];
  if (!scenario) {
    return null;
  }
  return (
    <section
      id="execution-comparison"
      className="mx-auto w-[calc(100%-4rem)] max-w-6xl pb-28 max-[760px]:w-[calc(100%-2rem)]"
      aria-labelledby="execution-comparison-heading"
    >
      <div className="mx-auto mb-10 max-w-2xl text-center">
        <p className="text-[10px] tracking-[.16em] text-stone-400 uppercase">
          Less rebuilding. More doing.
        </p>
        <h2
          id="execution-comparison-heading"
          className="font-display mt-5 text-[clamp(32px,4.5vw,55px)] leading-tight font-semibold tracking-[-.05em]"
        >
          Skip the blank page.
          <br />
          <span className="text-stone-400">Keep the work moving.</span>
        </h2>
        <p className="mt-5 text-sm leading-7 text-stone-500">
          A capable model plans the work. Hive supplies reusable capabilities.
          Your agent spends more time on the parts that are yours.
        </p>
      </div>
      <div className="overflow-hidden rounded-3xl border border-stone-200 bg-[#fafbf7]">
        <div
          className="flex flex-wrap gap-2 border-b border-stone-200 p-4 sm:p-6"
          role="group"
          aria-label="Choose a workflow"
        >
          {scenarios.map((item, position) => (
            <button
              type="button"
              key={item.short}
              aria-pressed={index === position}
              onClick={() => setIndex(position)}
              className={
                "cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45] " +
                `rounded-lg px-4 py-2.5 text-xs transition-colors ${index === position ? "bg-[#26351e] text-white" : "bg-white text-stone-500 hover:bg-stone-100"}`
              }
            >
              {item.short}
            </button>
          ))}
        </div>
        <div className="grid gap-8 p-5 sm:p-8 lg:grid-cols-[1.45fr_1fr] lg:gap-12">
          <figure className="min-w-0" aria-labelledby="workflow-chart-title">
            <p className="text-[10px] tracking-widest text-stone-400 uppercase">
              Scenario projection
            </p>
            <h3
              id="workflow-chart-title"
              className="font-display mt-3 text-lg font-semibold tracking-tight"
            >
              {scenario.name}
            </h3>
            <p className="mt-2 text-xs leading-6 text-stone-400">
              {scenario.context}
            </p>
            <div className="mt-6 grid grid-cols-2 gap-5 border-y border-stone-200/70 py-5">
              <div>
                <p className="text-[10px] text-stone-400">From scratch</p>
                <p className="mt-1 text-2xl font-medium tracking-tight text-stone-500">
                  {formatTime(scenario.withoutHive)}
                </p>
              </div>
              <div>
                <p className="text-[10px] text-[#829166]">With Hive + agent</p>
                <p className="mt-1 text-2xl font-medium tracking-tight text-[#5f7a3c]">
                  {formatTime(scenario.withHive)}
                </p>
              </div>
            </div>
            {/* oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- This SVG chart needs image semantics. */}
            <svg
              viewBox="0 0 640 230"
              role="img"
              aria-label={`From scratch ${formatTime(scenario.withoutHive)}; with Hive ${formatTime(scenario.withHive)}. Projected workflow times.`}
              className="mt-8 block h-auto w-full overflow-visible"
            >
              {[0, 5, 10, 15, 20].map((minute) => (
                <g key={minute}>
                  <line
                    x1={30 + (minute / 20) * 550}
                    x2={30 + (minute / 20) * 550}
                    y1="20"
                    y2="193"
                    stroke="#e5e8dc"
                  />
                  <text
                    x={30 + (minute / 20) * 550}
                    y="220"
                    fill="#9aa08f"
                    fontSize="11"
                    textAnchor="middle"
                  >
                    {minute}m
                  </text>
                </g>
              ))}
              <text x="30" y="40" fill="#7c8275" fontSize="12">
                From scratch
              </text>
              <rect
                x="30"
                y="53"
                width={width(scenario.withoutHive)}
                height="33"
                rx="5"
                fill="#b5bcaa"
              />
              <text
                x={40 + width(scenario.withoutHive)}
                y="75"
                fill="#68705d"
                fontSize="12"
              >
                {formatTime(scenario.withoutHive)}
              </text>
              <text x="30" y="124" fill="#586d3d" fontSize="12">
                With Hive + your agent
              </text>
              <rect
                x="30"
                y="138"
                width={width(scenario.withHive)}
                height="33"
                rx="5"
                fill="#8fa85d"
              />
              <text
                x={40 + width(scenario.withHive)}
                y="160"
                fill="#536b31"
                fontSize="12"
              >
                {formatTime(scenario.withHive)}
              </text>
            </svg>
            <figcaption className="mt-4 text-[11px] leading-6 text-stone-400">
              Planning figures for these scenarios, rather than recorded
              execution timings. Scaffolding assumes an existing suitable
              starter; your agent handles customization and local checks.
            </figcaption>
          </figure>
          <div className="rounded-2xl border border-[#dfe5d5] bg-white p-5 sm:p-6">
            <p className="text-[10px] tracking-widest text-stone-400 uppercase">
              The workflow walkthrough
            </p>
            <div className="mt-6 space-y-6">
              {scenario.steps.map((step, position) => (
                <div key={step} className="flex items-start gap-3">
                  <span className="mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-full bg-[#eff3e7] font-mono text-[9px] text-[#829361]">
                    {position + 1}
                  </span>
                  <p className="text-xs leading-6 text-stone-500">{step}</p>
                </div>
              ))}
            </div>
            <div className="mt-7 border-t border-stone-100 pt-5">
              <p className="text-xs font-medium text-[#52663c]">
                Same version. Same input. Same tool output.
              </p>
              <p className="mt-2 text-[11px] leading-6 text-stone-400">
                Codex, Claude Code or OpenCode can call Hive. Model choice
                changes the reasoning around the tool; it does not replace the
                tool&apos;s implementation.
              </p>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
export default ExecutionComparison;
