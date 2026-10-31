"use client";

import { Icon } from "../../../components/icon";
import { useLoopInView } from "../../../lib/hooks/use-loop-in-view";
import { SceneHexagon } from "./scene-hexagon";

const cellDelays = [
  "[--cell:0]",
  "[--cell:1]",
  "[--cell:2]",
  "[--cell:3]",
  "[--cell:4]",
  "[--cell:5]",
  "[--cell:6]",
  "[--cell:7]",
  "[--cell:8]",
  "[--cell:9]",
  "[--cell:10]",
  "[--cell:11]",
];
const inputs = ["report.pdf", "invoice.pdf", "lease.pdf"];
const tables = [
  {
    cells: 6,
    layout:
      "[grid-template-columns:repeat(2,1fr)] [grid-template-rows:repeat(3,1fr)]",
  },
  {
    cells: 8,
    layout:
      "[grid-template-columns:repeat(4,1fr)] [grid-template-rows:repeat(2,1fr)]",
  },
  {
    cells: 12,
    layout:
      "[grid-template-columns:repeat(3,1fr)] [grid-template-rows:repeat(4,1fr)]",
  },
];
const totals = ["$0.05", "$0.10", "$0.15"];

export const InvokeScene = () => {
  const ref = useLoopInView();
  return (
    <figure
      ref={ref}
      aria-label="Report, invoice, and lease PDFs pass through the same hosted extract_tables capability, producing different tables. Total cost rises from five to fifteen cents."
      data-loop="idle"
      className="hive-scene font-body invoke-scene [container-type:inline-size] relative m-0 [aspect-ratio:4_/_3] w-full shrink-0 overflow-hidden rounded-[var(--radius-3xl)] text-[11px] leading-[1.35] text-[color:var(--color-neutral-600)] [--scene-duration:7s] [--scene-ease:cubic-bezier(0.22,_1,_0.36,_1)] [--scene-story:scene-story-seven] [background:linear-gradient(_160deg,_var(--color-stone-50),_var(--color-stone-100)_)] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.input-chip-0]:[animation:invoke-input-first_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.input-chip-1]:[animation:invoke-input-second_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.input-chip-2]:[animation:invoke-input-third_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.invoke-node]:[animation:invoke-pulse_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.invoke-ring]:[animation:invoke-ring_7s_ease-in-out_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.invoke-table-0]:[animation:invoke-output-first_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.invoke-table-1]:[animation:invoke-output-second_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.invoke-table-2]:[animation:invoke-output-third_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.invoke-total-0]:[animation:invoke-price-first_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.invoke-total-1]:[animation:invoke-price-second_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.invoke-total-2]:[animation:invoke-price-third_7s_var(--scene-ease)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.scene-track]:[animation:var(--scene-story)_var(--scene-duration)_linear_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.table-cell-0]:[animation:invoke-cell-first_7s_var(--scene-ease)_calc(var(--cell)_*_40ms)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.table-cell-1]:[animation:invoke-cell-second_7s_var(--scene-ease)_calc(var(--cell)_*_40ms)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&:is([data-loop='running'],_[data-loop='paused'])_.table-cell-2]:[animation:invoke-cell-third_7s_var(--scene-ease)_calc(var(--cell)_*_40ms)_infinite_both] [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='idle']_.scene-track]:opacity-[0] [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='paused']_*]:[animation-play-state:paused]! [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='paused']_*::before]:[animation-play-state:paused]! [@media_(prefers-reduced-motion:no-preference)]:[&[data-loop='reset']_.scene-track]:opacity-[0]"
    >
      <div
        className="scene-track invoke-board absolute top-0 right-0 bottom-0 left-0"
        aria-hidden="true"
      >
        <div className="invoke-caption absolute top-[12%] right-[6%] left-[6%] flex justify-between gap-2 text-[11px] text-[color:var(--color-neutral-400)]">
          <span>Fresh inputs</span>
          <span>Structured output</span>
        </div>
        <svg
          className="invoke-wires absolute top-0 right-0 bottom-0 left-0 h-full w-full text-[color:var(--color-neutral-300)]"
          viewBox="0 0 320 240"
          preserveAspectRatio="none"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="M28 112H128M176 112H284"
            stroke="currentColor"
            strokeWidth="1.2"
          />
        </svg>
        <div className="invoke-input-lane absolute top-[47%] left-[4%] h-[30px] w-[34%] [transform:translateY(-50%)]">
          {inputs.map((input, index) => (
            <div
              key={input}
              className={`scene-surface input-chip absolute top-0 left-0 flex items-center gap-1 rounded-[var(--radius-xl)] pt-[7px] pr-1.5 pb-[7px] pl-1.5 text-[11.5px] whitespace-nowrap opacity-[0] [box-shadow:inset_0_1px_0_var(--color-white),_var(--shadow-sm),_var(--shadow-soft)] [background:var(--color-white)] [&.input-chip-2]:opacity-[1] [&>svg]:shrink-0 input-chip-${index}`}
            >
              <Icon name="file" size={14} />
              <span>{input}</span>
            </div>
          ))}
        </div>
        <div className="invoke-capability absolute top-[47%] left-[46%] [aspect-ratio:1] w-[22%] [transform:translate(-50%,_-50%)]">
          <div className="invoke-node relative flex h-full w-full items-center justify-center [&>svg:last-child]:relative [&>svg:last-child]:text-[color:var(--color-amber-800)]">
            <SceneHexagon className="invoke-node-shape absolute top-0 right-0 bottom-0 left-0 text-[color:var(--color-amber-200)] [filter:drop-shadow(0_2px_2px_rgb(0_0_0_/_0.05))]" />
            <span className="invoke-ring absolute top-[-3%] right-[-3%] bottom-[-3%] left-[-3%] rounded-full opacity-[0] [border:1px_solid_var(--color-amber-400)]" />
            <Icon name="code" size={23} />
          </div>
          <span className="invoke-node-label absolute top-[calc(100%_+_7px)] left-[50%] [transform:translateX(-50%)] text-[11px] font-medium whitespace-nowrap text-[color:var(--color-neutral-700)]">
            extract_tables
          </span>
          <span className="invoke-hosted scene-mono absolute top-[calc(100%_+_25px)] left-[50%] [transform:translateX(-50%)] font-mono text-[11px] text-[color:var(--color-neutral-400)]">
            hosted
          </span>
        </div>
        <div className="scene-surface invoke-output absolute top-[30%] right-[4%] flex h-[48%] w-[32%] flex-col rounded-[var(--radius-xl)] px-2 py-2.5 shadow-[inset_0_1px_0_var(--color-white),_var(--shadow-sm),_var(--shadow-soft)] [background:var(--color-white)]">
          <span className="invoke-output-title text-[11px] font-medium">
            Tables
          </span>
          <div className="invoke-table-area relative mx-0 my-[9px] min-h-0 [flex:1]">
            {tables.map((table, index) => (
              <div
                key={table.cells}
                className={`invoke-table absolute top-0 right-0 bottom-0 left-0 grid gap-[3px] opacity-[0] [&_.invoke-table-2_.table-cell:nth-child(-n+3)]:[background:var(--color-amber-200)] [&.invoke-table-1_.table-cell:nth-child(-n+4)]:[background:var(--color-amber-200)] [&.invoke-table-2]:opacity-[1] invoke-table-${index} ${
                  table.layout
                }`}
              >
                {Array.from({ length: table.cells }, (_, cell) => (
                  <span
                    key={`${table.cells}-${cell}`}
                    className={`table-cell rounded-[3px] [background:var(--color-neutral-200)] [&:nth-child(-n+2)]:[background:var(--color-amber-200)] table-cell-${index} ${
                      cellDelays[cell] ?? ""
                    }`}
                  />
                ))}
              </div>
            ))}
          </div>
          <div className="invoke-billing flex justify-center">
            <span className="scene-done scene-mono invoke-price relative inline-flex h-[22px] w-[54px] items-center justify-center gap-[3px] overflow-hidden rounded-[var(--radius-md)] px-1.5 py-0.5 font-mono text-[11px] text-[12px] whitespace-nowrap text-[color:var(--color-amber-800)] [background:var(--color-amber-100)]">
              {totals.map((total, index) => (
                <span
                  key={total}
                  className={`invoke-total absolute top-0 right-0 bottom-0 left-0 flex items-center justify-center opacity-[0] [&.invoke-total-2]:opacity-[1] invoke-total-${index}`}
                >
                  {total}
                </span>
              ))}
            </span>
          </div>
        </div>
      </div>
    </figure>
  );
};

export default InvokeScene;
