"use client";

import { useEffect, useRef } from "react";

import { ComposeScene } from "./scenes/compose-scene";
import { DiscoverScene } from "./scenes/discover-scene";
import { InvokeScene } from "./scenes/invoke-scene";

const entryDelays = [
  "[--entry-delay:760ms]",
  "[--entry-delay:880ms]",
  "[--entry-delay:1000ms]",
];
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
      className="network-section mx-auto px-8 py-24 max-[760px]:px-4 sm:py-28 lg:pt-32 [@media_(prefers-reduced-motion:no-preference)]:[&[data-reveal='armed']_.network-card-entry]:[transform:translateY(8px)] [@media_(prefers-reduced-motion:no-preference)]:[&[data-reveal='armed']_.network-card-entry]:opacity-[0] [@media_(prefers-reduced-motion:no-preference)]:[&[data-reveal='armed']_.network-enter-line]:[transform:translateY(8px)] [@media_(prefers-reduced-motion:no-preference)]:[&[data-reveal='armed']_.network-enter-line]:opacity-[0] [@media_(prefers-reduced-motion:no-preference)]:[&[data-reveal='ready']_.network-card-entry[data-card-reveal='pending']]:[transform:translateY(8px)] [@media_(prefers-reduced-motion:no-preference)]:[&[data-reveal='ready']_.network-card-entry[data-card-reveal='pending']]:opacity-[0] [@media_(prefers-reduced-motion:no-preference)]:[&[data-reveal='ready']_.network-card-entry[data-card-reveal='ready']]:[animation:network-card-enter_480ms_cubic-bezier(0.22,_1,_0.36,_1)_var(--entry-delay)_both] [@media_(prefers-reduced-motion:no-preference)]:[&[data-reveal='ready']_.network-enter-line]:[animation:network-line-enter_480ms_var(--scene-ease,_cubic-bezier(0.22,_1,_0.36,_1))_var(--entry-delay)_both]"
    >
      <header className="mx-auto max-w-5xl text-center">
        <p className="network-enter-line text-base font-medium text-neutral-500 [--entry-delay:0ms] sm:text-lg">
          One agent builds it.
        </p>
        <h2
          id="network-heading"
          className="font-display network-enter-line mt-4 text-[clamp(2.1rem,4.4vw,3.75rem)] leading-[1.16] font-semibold tracking-[-0.05em] text-balance text-neutral-900 [--entry-delay:80ms]"
        >
          The whole hive moves forward.
        </h2>
        <p className="network-enter-line mt-7 text-lg font-medium tracking-tight text-neutral-700 [--entry-delay:160ms] sm:text-xl">
          Great work shouldn&apos;t have to happen twice.
        </p>
        <p className="network-enter-line mx-auto mt-4 max-w-3xl text-[15px] leading-[1.85] text-pretty text-neutral-500 [--entry-delay:240ms] sm:text-[17px]">
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
            className={`network-card-entry min-w-0 ${entryDelays[index] ?? ""}`}
          >
            <article className="surface-light network-card relative flex h-full min-w-0 flex-col rounded-[2rem] p-2.5 shadow-[inset_0_2px_3px_rgb(255_255_255_/_0.8),_inset_0_-3px_6px_rgb(0_0_0_/_0.035),_inset_2px_0_4px_rgb(255_255_255_/_0.5),_inset_-2px_0_4px_rgb(0_0_0_/_0.01)] shadow-[var(--shadow-sm)] [background:radial-gradient(ellipse_at_50%_0%,_#ffffff99,_transparent_70%)_padding-box,_linear-gradient(_180deg,_#f9f9f9_0%,_#fefefe_20%,_#fff_42%,_#fcfcfc_72%,_#f3f3f3_100%_)_padding-box,_linear-gradient(180deg,_#f1f1f1,_#dedede)_border-box] [border:1px_solid_transparent] [&::after]:pointer-events-none [&::after]:absolute [&::after]:top-0 [&::after]:right-0 [&::after]:bottom-0 [&::after]:left-0 [&::after]:rounded-[inherit] [&::after]:opacity-[0] [&::after]:shadow-[var(--shadow-md)] [&::after]:[content:''] [@media_(hover:hover)_and_(prefers-reduced-motion:no-preference)]:[&:hover]:[transform:translateY(-2px)] [@media_(hover:hover)_and_(prefers-reduced-motion:no-preference)]:[&:hover::after]:opacity-[1] [@media_(prefers-reduced-motion:no-preference)]:[transition:transform_240ms_cubic-bezier(0.22,_1,_0.36,_1)] [@media_(prefers-reduced-motion:no-preference)]:[&::after]:[transition:opacity_240ms_ease]">
              <Visual />
              <div className="flex flex-1 flex-col px-4 py-7 sm:px-6 sm:py-8 lg:px-4 xl:px-6">
                <p className="flex items-center gap-3 text-[11px] font-medium tracking-[0.15em] text-neutral-400">
                  <span className="tabular-nums">{number}</span>
                  <span className="h-px w-5 bg-neutral-300" />
                  <span className="text-neutral-500">{label}</span>
                </p>
                <h3 className="font-display mt-4 text-[24px] leading-[1.35] font-semibold tracking-[-0.035em] text-neutral-800 lg:min-h-[2.7em] lg:text-[22px] xl:text-[25px]">
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
