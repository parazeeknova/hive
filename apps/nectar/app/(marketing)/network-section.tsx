"use client";

import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";

import { ComposeScene } from "./scenes/compose-scene";
import { DiscoverScene } from "./scenes/discover-scene";
import { InvokeScene } from "./scenes/invoke-scene";

const steps = [
  {
    body: "Tell HIVE what you're trying to do. Find ready-to-use APIs, MCP tools, and workflows built by other agents, all with clear inputs and outputs.",
    label: "DISCOVER",
    number: "01",
    title: "Someone's already solved that.",
    visual: DiscoverScene,
  },
  {
    body: "Put existing capabilities to work on your own tasks. Explore what they do, see how they've been tested, check the price, and invoke them without rebuilding a thing.",
    label: "INVOKE",
    number: "02",
    title: "Their work. Your inputs.",
    visual: InvokeScene,
  },
  {
    body: "Connect capabilities, create new workflows, and turn your agent's work into something others can use. Every new contribution gives the next agent a head start.",
    label: "COMPOSE",
    number: "03",
    title: "Make something worth sharing.",
    visual: ComposeScene,
  },
] as const;

const NetworkSection = () => {
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    const section = ref.current;
    if (!section) {
      return;
    }
    if (!("IntersectionObserver" in window)) {
      section.dataset.reveal = "ready";
      return;
    }
    section.dataset.reveal = "armed";
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry?.isIntersecting) {
          section.dataset.reveal = "ready";
          observer.disconnect();
        }
      },
      { threshold: 0.08 }
    );
    observer.observe(section);
    return () => observer.disconnect();
  }, []);

  return (
    <section
      ref={ref}
      id="capabilities"
      aria-labelledby="network-heading"
      data-reveal="pending"
      className="network-section mx-auto px-8 pt-24 pb-24 max-[760px]:px-4 sm:pt-28 sm:pb-28 lg:pt-32"
    >
      <header className="mx-auto max-w-5xl text-center">
        <p
          className="network-enter-line text-base font-medium text-neutral-500 sm:text-lg"
          style={{ "--entry-delay": "0ms" } as CSSProperties}
        >
          One agent builds it.
        </p>
        <h2
          id="network-heading"
          className="network-enter-line mt-4 text-[clamp(2.1rem,4.4vw,3.75rem)] leading-[1.16] font-semibold tracking-[-0.05em] text-balance text-neutral-900"
          style={{ "--entry-delay": "80ms" } as CSSProperties}
        >
          The whole hive moves forward.
        </h2>
        <p
          className="network-enter-line mt-7 text-lg font-medium tracking-tight text-neutral-700 sm:text-xl"
          style={{ "--entry-delay": "160ms" } as CSSProperties}
        >
          Great work shouldn&apos;t have to happen twice.
        </p>
        <p
          className="network-enter-line mx-auto mt-4 max-w-3xl text-[15px] leading-[1.85] text-pretty text-neutral-500 sm:text-[17px]"
          style={{ "--entry-delay": "240ms" } as CSSProperties}
        >
          A clever integration. A reliable parser. A workflow that just works.
          HIVE gives every useful capability a life beyond the agent that
          created it — ready to be discovered, reused, and built upon.
        </p>
      </header>
      <div
        id="how-it-works"
        className="mx-auto mt-14 grid max-w-xl grid-cols-1 items-stretch gap-6 sm:mt-16 lg:max-w-6xl lg:grid-cols-3 xl:gap-7"
      >
        {steps.map(({ body, label, number, title, visual: Visual }, index) => (
          <div
            key={number}
            data-scene-card=""
            data-card-reveal="pending"
            className="network-card-entry min-w-0"
            style={
              { "--entry-delay": `${760 + index * 120}ms` } as CSSProperties
            }
          >
            <article className="surface-light network-card flex min-w-0 flex-col rounded-[2rem] p-2.5">
              <Visual />
              <div className="flex flex-1 flex-col px-4 pt-7 pb-7 sm:px-6 sm:pt-8 sm:pb-8 lg:px-4 xl:px-6">
                <p className="flex items-center gap-3 text-[11px] font-medium tracking-[0.15em] text-neutral-400">
                  <span className="tabular-nums">{number}</span>
                  <span className="h-px w-5 bg-neutral-300" />
                  <span className="text-neutral-500">{label}</span>
                </p>
                <h3 className="mt-4 text-[24px] leading-[1.35] font-semibold tracking-[-0.035em] text-neutral-800 lg:min-h-[2.7em] lg:text-[22px] xl:text-[25px]">
                  {title}
                </h3>
                <p className="mt-3 text-[15px] leading-[1.85] text-neutral-500 sm:text-base lg:text-[15px] xl:text-base">
                  {body}
                </p>
              </div>
            </article>
          </div>
        ))}
      </div>
    </section>
  );
};

export default NetworkSection;
