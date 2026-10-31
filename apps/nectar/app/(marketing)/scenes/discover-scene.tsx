"use client";

import { Icon } from "../../../components/icon";
import { useLoopInView } from "../../../lib/hooks/use-loop-in-view";
import { SceneHexagon } from "./scene-hexagon";

const query = "Extract tables from PDFs";
const results = [
  { name: "pdf_table_extract", price: "$0.05", tag: "MCP" },
  { name: "doc_parser", price: "$0.02", tag: "API" },
  { name: "invoice_reader", price: "$0.03", tag: "Workflow" },
];

export const DiscoverScene = () => {
  const ref = useLoopInView();
  return (
    <figure
      ref={ref}
      aria-label="Search for PDF table extraction, compare three capabilities, and select the tested MCP tool at five cents per call."
      data-loop="idle"
      className="hive-scene font-body discover-scene [container-type:inline-size] relative m-0 [aspect-ratio:4_/_3] w-full shrink-0 overflow-hidden rounded-[var(--radius-3xl)] text-[11px] leading-[1.35] text-[color:var(--color-neutral-600)] [--scene-duration:6s] [--scene-duration:7s] [--scene-ease:cubic-bezier(0.22,_1,_0.36,_1)] [--scene-story:scene-story-seven] [--scene-story:scene-story-six] [background:linear-gradient(_160deg,_var(--color-stone-50),_var(--color-stone-100)_)] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.discover-caret]:[animation:discover-caret_6s_steps(1,_end)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.discover-highlight]:[animation:discover-select_6s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.discover-row-0]:[animation:discover-row-first_6s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.discover-row-1]:[animation:discover-row-second_6s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.discover-row-2]:[animation:discover-row-third_6s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.discover-tested]:[animation:discover-tested_6s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.discover-type-mask]:[animation:discover-type_6s_steps(24,_end)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.scene-track]:[animation:var(--scene-story)_var(--scene-duration)_linear_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='idle']_.scene-track]:opacity-[0] [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='paused']_*]:[animation-play-state:paused]! [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='paused']_*::before]:[animation-play-state:paused]! [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='reset']_.scene-track]:opacity-[0]"
    >
      <div
        className="scene-track discover-board absolute top-0 top-[4%] right-0 right-[5%] bottom-0 bottom-[4%] left-0 left-[5%] flex flex-col justify-center gap-[clamp(6px,_2.5cqw,_12px)]"
        aria-hidden="true"
      >
        <div className="scene-surface discover-search flex h-[clamp(32px,_10cqw,_44px)] min-h-8 shrink-0 items-center gap-2 rounded-[var(--radius-xl)] px-2.5 py-0 text-[color:var(--color-neutral-400)] shadow-[inset_0_1px_0_var(--color-white),_var(--shadow-sm),_var(--shadow-soft)] [background:var(--color-white)] [&>svg]:shrink-0">
          <Icon name="search" size={16} />
          <span className="discover-query relative inline-block overflow-hidden font-mono text-[11px] leading-[16px] whitespace-nowrap text-[color:var(--color-neutral-600)]">
            {query}
            <span className="discover-type-mask absolute top-0 right-0 bottom-0 left-0 [transform:translateX(100%)] [background:var(--color-white)]">
              <span className="discover-caret absolute top-px left-0 h-3.5 w-px opacity-[0] [background:var(--color-neutral-500)]" />
            </span>
          </span>
        </div>
        <div className="discover-results grid max-h-[220px] min-h-0 [flex:1] grid-rows-[repeat(3,_minmax(0,_1fr))] gap-[clamp(5px,_2cqw,_9px)]">
          {results.map((result, index) => (
            <div
              key={result.name}
              className={`discover-row relative min-h-0 rounded-[var(--radius-xl)] [box-shadow:var(--shadow-xs)] [background:var(--color-white)] [&_.discover-row-2]:opacity-[0.45] [&.discover-row-0_.discover-hex]:text-[color:var(--color-amber-200)] [&.discover-row-1]:opacity-[0.45] discover-row-${index}`}
            >
              {index === 0 && (
                <div className="discover-highlight absolute top-0 right-0 bottom-0 left-0 rounded-[inherit] shadow-[inset_0_0_0_1px_var(--color-amber-300)] [background:var(--color-amber-50)]" />
              )}
              <div className="discover-row-content relative flex h-full items-center gap-2 px-[9px] py-1">
                <SceneHexagon className="discover-hex h-[clamp(22px,_7cqw,_30px)] w-[clamp(22px,_7cqw,_30px)] shrink-0 text-[color:var(--color-neutral-100)]" />
                <div className="discover-name flex min-w-0 [flex:1] flex-col gap-[3px] text-[clamp(11px,_3cqw,_13px)] font-medium text-[color:var(--color-neutral-700)] [&>span]:whitespace-nowrap">
                  <span>{result.name}</span>
                  <div className="discover-tags flex items-center gap-1">
                    <span className="scene-tag inline-flex items-center rounded-[var(--radius-sm)] px-[5px] py-px font-mono text-[11px] text-[color:var(--color-neutral-500)] [background:var(--color-neutral-100)]">
                      {result.tag}
                    </span>
                    {index === 0 && (
                      <span className="scene-done discover-tested inline-flex items-center justify-center gap-[3px] rounded-[var(--radius-md)] px-1 py-px text-[12px] whitespace-nowrap text-[color:var(--color-amber-800)] [background:var(--color-amber-100)]">
                        <Icon name="check" size={11} />
                        Tested
                      </span>
                    )}
                  </div>
                </div>
                <span className="scene-mono discover-price ml-auto shrink-0 font-mono text-[11px] text-[color:var(--color-neutral-500)]">
                  {result.price}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </figure>
  );
};

export default DiscoverScene;
