"use client";

import "./execution-comparison.css";
import { useState } from "react";
import type { CSSProperties } from "react";

type Series = "both" | "without" | "with";
const scenarios = [
  {
    context: "New integration",
    name: "Repository stack report",
    withHive: 8,
    withoutHive: 60,
  },
  {
    context: "New workflow",
    name: "Dependency + license report",
    withHive: 15,
    withoutHive: 120,
  },
  {
    context: "10 documents",
    name: "PDF table extraction",
    withHive: 25,
    withoutHive: 180,
  },
  {
    context: "Warm reuse",
    name: "Existing local analyzer",
    withHive: 8,
    withoutHive: 3,
  },
];
// The longest supplied estimate is 180s; a 200s axis keeps every bar in view.
const axisMax = 200;

const ExecutionComparison = () => {
  const [focus, setFocus] = useState<Series>("both");
  const highlight = (series: Exclude<Series, "both">) =>
    setFocus((current) => (current === series ? "both" : series));

  return (
    <section
      id="execution-comparison"
      aria-labelledby="execution-comparison-heading"
      className="mx-auto w-[calc(100%-4rem)] max-w-6xl pt-3 pb-24 max-[760px]:w-[calc(100%-2rem)] sm:pb-28"
    >
      <header className="mx-auto max-w-3xl text-center">
        <p className="text-[11px] font-medium tracking-[0.15em] text-neutral-500 uppercase">
          Less time starting from scratch
        </p>
        <h2
          id="execution-comparison-heading"
          className="mt-5 text-[clamp(2.1rem,4.4vw,3.75rem)] leading-[1.16] font-semibold tracking-[-0.05em] text-balance text-neutral-900"
        >
          Skip the rebuild.
          <span className="block text-neutral-400">Get to the result.</span>
        </h2>
        <p className="mx-auto mt-6 max-w-xl text-[15px] leading-[1.85] text-neutral-500 sm:text-base">
          A ready-made capability can give your agent a head start. Here&apos;s
          how elapsed time compares across four assumed workloads.
        </p>
      </header>
      <div
        className="surface-light execution-comparison-card mt-10 rounded-[2rem] p-5 sm:mt-12 sm:rounded-[2.5rem] sm:p-8 lg:p-10"
        data-focus={focus}
      >
        <div className="comparison-chart-heading">
          <div>
            <h3 id="comparison-chart-title">Time to result</h3>
            <p>Elapsed time in seconds · lower is faster</p>
          </div>
          <div
            className="comparison-legend"
            aria-label="Highlight a comparison series"
          >
            <button
              type="button"
              aria-pressed={focus === "without"}
              onClick={() => highlight("without")}
            >
              <span
                className="comparison-key comparison-key-without"
                aria-hidden="true"
              />
              Without HIVE
            </button>
            <button
              type="button"
              aria-pressed={focus === "with"}
              onClick={() => highlight("with")}
            >
              <span
                className="comparison-key comparison-key-with"
                aria-hidden="true"
              />
              With HIVE
            </button>
          </div>
        </div>
        <figure
          className="comparison-figure"
          aria-labelledby="comparison-chart-title"
          aria-describedby="comparison-chart-caption"
        >
          <div className="comparison-axis" aria-hidden="true">
            <span>Assumed workload</span>
            <div>
              {[0, 50, 100, 150, 200].map((value) => (
                <span key={value}>
                  {value}
                  {value === axisMax ? "s" : ""}
                </span>
              ))}
            </div>
          </div>
          <dl className="comparison-scenarios">
            {scenarios.map(({ context, name, withHive, withoutHive }) => (
              <div key={name} className="comparison-scenario">
                <dt>
                  <strong>{name}</strong>
                  <span>{context}</span>
                </dt>
                <dd>
                  <div className="comparison-bars">
                    <div
                      className="comparison-bar-row comparison-series-without"
                      style={
                        {
                          "--bar-width": `${(withoutHive / axisMax) * 100}%`,
                        } as CSSProperties
                      }
                    >
                      <span className="comparison-bar" aria-hidden="true" />
                      <span className="comparison-bar-value">
                        <span className="sr-only">Without HIVE: </span>
                        {withoutHive}s
                      </span>
                    </div>
                    <div
                      className="comparison-bar-row comparison-series-with"
                      style={
                        {
                          "--bar-width": `${(withHive / axisMax) * 100}%`,
                        } as CSSProperties
                      }
                    >
                      <span className="comparison-bar" aria-hidden="true" />
                      <span className="comparison-bar-value">
                        <span className="sr-only">With HIVE: </span>
                        {withHive}s
                      </span>
                    </div>
                  </div>
                </dd>
              </div>
            ))}
          </dl>
          <figcaption
            id="comparison-chart-caption"
            className="comparison-chart-caption"
          >
            <span>Illustrative estimates</span>
            Assumed workloads and times, not measurements or forecasts. Existing
            local tooling may remain faster. Fixed model: GPT-6.1 Sol · medium.
          </figcaption>
        </figure>
      </div>
    </section>
  );
};

export default ExecutionComparison;
