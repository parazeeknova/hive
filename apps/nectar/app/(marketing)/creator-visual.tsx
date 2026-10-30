"use client";

import "./creator-visual.css";
import { Icon } from "../../components/icon";
import { useLoopInView } from "../../lib/hooks/use-loop-in-view";
import { SceneHexagon } from "./scenes/scene-hexagon";

const calls = [
  { input: "repo-a / main", output: "Stack report A" },
  { input: "repo-b / release", output: "Stack report B" },
  { input: "repo-c / v2.0", output: "Stack report C" },
];

const CreatorVisual = () => {
  const ref = useLoopInView();

  return (
    <figure
      ref={ref}
      data-loop="idle"
      className="publisher-stage m-0 min-w-0 overflow-hidden rounded-[1.75rem] bg-linear-160 from-stone-50 to-stone-100 p-[clamp(14px,3vw,36px)]"
      aria-label="Illustrative publishing activity: one Repository Analyzer capability serves three different repositories at five cents per call, for fifteen cents in total call value."
    >
      <div
        className="publisher-story mx-auto w-full max-w-[420px]"
        aria-hidden="true"
      >
        <div className="mb-5 flex items-center justify-between gap-3 px-1 text-[11px] text-neutral-400">
          <span className="font-medium">One implementation. Fresh inputs.</span>
          <span className="font-mono">EXAMPLE</span>
        </div>
        <div className="rounded-3xl bg-white p-4 shadow-[inset_0_1px_0_var(--color-white),var(--shadow-sm),var(--shadow-soft)] sm:p-6">
          <div className="flex items-center gap-3">
            <span className="relative block size-11 shrink-0">
              <SceneHexagon className="size-full text-amber-200" />
              <Icon
                name="code"
                size={20}
                className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-amber-900"
              />
            </span>
            <div className="min-w-0">
              <p className="text-[13px] leading-snug font-semibold text-neutral-800 sm:text-[15px]">
                Repository Analyzer
              </p>
              <p className="mt-1 font-mono text-[11px] text-neutral-400">
                repository_analyzer / v1.0
              </p>
            </div>
          </div>
          <div className="mt-5 flex items-center justify-between gap-3">
            <span className="inline-flex items-center gap-1.5 rounded-full bg-amber-50 px-2.5 py-1.5 text-[11px] font-medium text-amber-800">
              <Icon name="check" size={13} /> Published
            </span>
            <p className="text-[11px] text-neutral-400">
              <span className="font-mono text-neutral-700">$0.05</span> / call
            </p>
          </div>
          <div className="mt-5 border-t border-neutral-100 pt-5">
            <div className="mb-2 grid grid-cols-[minmax(0,1fr)_13px_minmax(0,1fr)] items-center gap-1.5 px-2 font-mono text-[11px] text-neutral-400">
              <span>INPUT</span>
              <span className="col-span-2 text-right">OUTPUT</span>
            </div>
            <div className="space-y-2">
              {calls.map(({ input, output }, index) => (
                <div
                  key={input}
                  className={`publisher-call publisher-call-${index} grid grid-cols-[minmax(0,1fr)_13px_minmax(0,1fr)] items-center gap-1.5 rounded-xl bg-neutral-50 px-2 py-3`}
                >
                  <span className="min-w-0 font-mono text-[11px] wrap-anywhere text-neutral-500">
                    {input}
                  </span>
                  <Icon name="arrow" size={13} className="text-neutral-300" />
                  <span className="min-w-0 text-right text-[11px] font-medium wrap-anywhere text-neutral-600 sm:text-xs">
                    {output}
                  </span>
                </div>
              ))}
            </div>
          </div>
          <div className="publisher-return mt-5 flex items-end justify-between gap-3 border-t border-neutral-100 pt-5">
            <div>
              <p className="text-[11px] font-medium text-neutral-500">
                Same tool. Three calls.
              </p>
              <p className="mt-1 text-[11px] text-neutral-400">
                Total example call value
              </p>
            </div>
            <span className="font-mono text-[25px] leading-none tracking-[-0.06em] text-neutral-800 sm:text-[28px]">
              $0.15
            </span>
          </div>
        </div>
        <p className="mt-5 text-center text-[11px] font-medium text-neutral-400">
          Hosted once. Ready for the next call.
        </p>
      </div>
    </figure>
  );
};

export default CreatorVisual;
