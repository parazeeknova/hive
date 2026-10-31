"use client";

import { useId } from "react";

import { Icon } from "../../../components/icon";
import type { IconName } from "../../../components/icon";
import { useLoopInView } from "../../../lib/hooks/use-loop-in-view";
import { SceneHexagon } from "./scene-hexagon";

const nodes: { icon: IconName; label: string }[] = [
  { icon: "globe", label: "Fetch" },
  { icon: "file", label: "Parse" },
  { icon: "layers", label: "Summarize" },
];

export const ComposeScene = () => {
  const ref = useLoopInView();
  const pattern = useId();
  return (
    <figure
      ref={ref}
      aria-label="Fetch, Parse, and Summarize hexagons combine into doc_to_summary version one at four cents, become published, and give the next capability a place to connect."
      data-loop="idle"
      className="hive-scene font-body compose-scene [container-type:inline-size] relative m-0 [aspect-ratio:4_/_3] w-full shrink-0 overflow-hidden rounded-[var(--radius-3xl)] text-[11px] leading-[1.35] text-[color:var(--color-neutral-600)] [--scene-duration:7s] [--scene-ease:cubic-bezier(0.22,_1,_0.36,_1)] [--scene-story:scene-story-seven] [background:linear-gradient(_160deg,_var(--color-stone-50),_var(--color-stone-100)_)] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-cluster]:[animation:compose-lift_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-cluster::before]:[animation:compose-depth_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-combined]:[animation:compose-combined_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-data-dot]:[animation:compose-data_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-line]:[animation:compose-line_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-next-node]:[animation:compose-next_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-next-slot]:[animation:compose-slot_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-node-0_.compose-token]:[animation:compose-fetch_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-node-1_.compose-token]:[animation:compose-parse_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-node-2_.compose-token]:[animation:compose-summarize_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-node-copy]:[animation:compose-labels_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-outline]:[animation:compose-outline_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-outline-fill]:[animation:compose-fill_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-published]:[animation:compose-published_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.compose-token>.scene-hexagon]:[animation:compose-hex-fade_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.scene-track]:[animation:var(--scene-story)_var(--scene-duration)_linear_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='idle']_.scene-track]:opacity-[0] [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='paused']_*]:[animation-play-state:paused]! [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='paused']_*::before]:[animation-play-state:paused]! [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='reset']_.scene-track]:opacity-[0]"
    >
      <div
        className="scene-track compose-board absolute top-0 right-0 bottom-0 left-0"
        aria-hidden="true"
      >
        <svg
          className="compose-grid absolute top-0 right-0 bottom-0 left-0 h-full w-full text-[color:var(--color-neutral-300)] opacity-[0.45]"
          viewBox="0 0 320 240"
          preserveAspectRatio="xMidYMid slice"
          fill="none"
          aria-hidden="true"
        >
          <defs>
            <pattern
              id={pattern}
              width="36"
              height="62.4"
              patternUnits="userSpaceOnUse"
            >
              <path
                d="M18 0 36 10.4V31.2L18 41.6 0 31.2V10.4ZM0 31.2V52L18 62.4 36 52V31.2"
                stroke="currentColor"
                strokeWidth=".7"
              />
            </pattern>
          </defs>
          <rect width="320" height="240" fill={`url(#${pattern})`} />
        </svg>
        <div className="compose-cluster absolute top-0 right-0 bottom-0 left-0 [transform:translateY(-4px)] [&::before]:absolute [&::before]:top-[9%] [&::before]:left-[19%] [&::before]:h-[69%] [&::before]:w-[52%] [&::before]:rounded-[var(--radius-3xl)] [&::before]:opacity-[0.5] [&::before]:shadow-[var(--shadow-md)] [&::before]:[content:'']">
          <svg
            className="compose-connections absolute top-0 right-0 bottom-0 left-0 h-full w-full text-[color:var(--color-amber-600)]"
            viewBox="0 0 320 240"
            preserveAspectRatio="none"
            fill="none"
            aria-hidden="true"
          >
            <path
              className="compose-line opacity-[0] [stroke-dasharray:100] [stroke-dashoffset:0]"
              d="M109 72H181L145 134"
              pathLength="100"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
            <circle
              className="compose-data-dot opacity-[0]"
              r="3"
              fill="currentColor"
            />
            <rect
              className="compose-outline-fill opacity-[0.08]"
              x="61"
              y="21"
              width="167"
              height="167"
              rx="23"
              fill="currentColor"
            />
            <rect
              className="compose-outline [stroke-dasharray:100] [stroke-dashoffset:0]"
              x="61"
              y="21"
              width="167"
              height="167"
              rx="23"
              pathLength="100"
              stroke="currentColor"
              strokeWidth="1.2"
            />
          </svg>
          {nodes.map((node, index) => (
            <div
              key={node.label}
              className={`compose-node absolute [aspect-ratio:1] w-[26%] [transform:translate(-50%,_-50%)] [&.compose-node-0]:top-[30%] [&.compose-node-0]:left-[34%] [&.compose-node-1]:top-[30%] [&.compose-node-1]:left-[56.5%] [&.compose-node-2]:top-[56%] [&.compose-node-2]:left-[45.25%] compose-node-${index}`}
            >
              <div className="compose-token relative h-full w-full [&>.scene-hexagon]:absolute [&>.scene-hexagon]:top-0 [&>.scene-hexagon]:right-0 [&>.scene-hexagon]:bottom-0 [&>.scene-hexagon]:left-0 [&>.scene-hexagon]:text-[color:var(--color-white)] [&>.scene-hexagon]:opacity-[0.35] [&>.scene-hexagon]:[filter:drop-shadow(0_1px_2px_rgb(0_0_0_/_0.06))]">
                <SceneHexagon />
                <div className="compose-node-copy absolute top-0 right-0 bottom-0 left-0 flex flex-col items-center justify-center gap-[3px] text-[11px] text-[color:var(--color-neutral-500)] opacity-[0]">
                  <Icon name={node.icon} size={17} />
                  <span>{node.label}</span>
                </div>
              </div>
            </div>
          ))}
          <div className="compose-combined absolute top-[40%] left-[45.25%] flex [transform:translate(-50%,_-50%)] flex-col items-center gap-2 text-[clamp(11px,_3.5cqw,_14px)] font-medium whitespace-nowrap text-[color:var(--color-neutral-700)] [&>div]:flex [&>div]:gap-3 [&>div]:text-[color:var(--color-neutral-500)]">
            <span>doc_to_summary</span>
            <div className="scene-mono font-mono text-[11px]">
              <span>v1</span>
              <span>$0.04</span>
            </div>
          </div>
        </div>
        <span className="scene-done compose-published absolute top-[81%] left-[45.25%] inline-flex [transform:translateX(-50%)] items-center justify-center gap-[3px] rounded-[var(--radius-md)] px-1.5 py-0.5 text-[12px] whitespace-nowrap text-[color:var(--color-amber-800)] [background:var(--color-amber-100)]">
          <Icon name="check" size={12} />
          Published
        </span>
        <div className="compose-next-slot absolute top-[56%] left-[68%] [aspect-ratio:1] w-[26%] [transform:translate(-50%,_-50%)] text-[color:var(--color-neutral-300)] opacity-[0] [&>svg]:h-full [&>svg]:w-full">
          <svg viewBox="0 0 100 100" fill="none" aria-hidden="true">
            <path
              d="m50 3 41 23.5v47L50 97 9 73.5v-47L50 3Z"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeDasharray="4 5"
            />
          </svg>
        </div>
        <div className="compose-next-node absolute top-[56%] left-[68%] [aspect-ratio:1] w-[26%] [transform:translate(-50%,_-50%)] [&>.scene-hexagon]:absolute [&>.scene-hexagon]:top-0 [&>.scene-hexagon]:right-0 [&>.scene-hexagon]:bottom-0 [&>.scene-hexagon]:left-0 [&>.scene-hexagon]:text-[color:var(--color-neutral-200)] [&>.scene-hexagon]:opacity-[0.6] [&>svg:last-child]:absolute [&>svg:last-child]:top-[50%] [&>svg:last-child]:left-[50%] [&>svg:last-child]:[transform:translate(-50%,_-50%)] [&>svg:last-child]:text-[color:var(--color-neutral-400)]">
          <SceneHexagon />
          <Icon name="plus" size={19} />
        </div>
      </div>
    </figure>
  );
};

export default ComposeScene;
