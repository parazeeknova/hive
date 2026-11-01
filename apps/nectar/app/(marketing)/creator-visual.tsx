"use client";

import { useState } from "react";

import { Icon } from "../../components/icon";
import { darkSurface, lightSurface } from "./surfaces";

const examples = [
  {
    calls: [
      { input: "0 failed · 92% coverage", output: "Ready to release" },
      { input: "2 critical findings", output: "Hold the release" },
      { input: "1 failed test", output: "Fix before shipping" },
    ],
    detail: "Turn CI signals into a repeatable release decision.",
    footer: "Pure JSON in. A clear decision out.",
    label: "Release checks",
    name: "Release readiness gate",
    runtime: "Hosted Wasm",
  },
  {
    calls: [
      { input: "Repository + full SHA", output: "Stack report" },
      { input: "Manifest files", output: "Dependency inventory" },
      { input: "Versioned workflow", output: "Combined report" },
    ],
    detail: "One call for a stack report and dependency inventory.",
    footer: "Compose capabilities. Reuse the workflow.",
    label: "Onboarding",
    name: "Repository onboarding dossier",
    runtime: "Workflow",
  },
  {
    calls: [
      { input: "Working directory", output: "Local approval" },
      { input: "Caller-owned connection", output: "Scoped file access" },
      { input: "Explicit arguments", output: "Recorded invocation" },
    ],
    detail: "Package a tool call with explicit connection bindings.",
    footer: "Your connection. Your approved directories.",
    label: "Connected MCP",
    name: "Private workspace tools",
    runtime: "Connected MCP",
  },
] as const;

const CreatorVisual = () => {
  const [selected, setSelected] = useState(0);
  const example = examples[selected] ?? examples[0];
  return (
    <figure
      className="m-0 min-w-0 rounded-[1.75rem] bg-linear-160 from-neutral-50 to-neutral-100 p-4 sm:p-7"
      aria-label="Explore examples of reusable Hive capabilities"
    >
      <fieldset
        className="mb-6 flex min-w-0 flex-wrap gap-1.5 border-0 p-0"
        aria-label="Choose a capability example"
      >
        {examples.map((item, index) => (
          <button
            key={item.name}
            type="button"
            onClick={() => setSelected(index)}
            aria-pressed={selected === index}
            className={`cursor-pointer rounded-full px-3 py-2 text-[10px] font-medium transition-[filter,box-shadow] ${selected === index ? `${darkSurface} text-white` : "text-neutral-500 hover:bg-white"}`}
          >
            {item.label}
          </button>
        ))}
      </fieldset>
      <div className="relative isolate pt-3">
        <div
          aria-hidden="true"
          className="absolute inset-x-5 top-0 -z-10 h-24 rounded-[22px] border border-neutral-200 bg-white/60"
        />
        <div
          aria-hidden="true"
          className="absolute inset-x-2.5 top-1.5 -z-10 h-24 rounded-[22px] border border-neutral-200 bg-white/80"
        />
        <div className={`relative rounded-[22px] p-5 sm:p-6 ${lightSurface}`}>
          <div className="flex items-center justify-between gap-3">
            <span
              className={`flex size-10 items-center justify-center rounded-xl text-white ${darkSurface}`}
            >
              <Icon name="code" size={19} />
            </span>
            <span className="rounded-full border border-neutral-200 bg-white px-2.5 py-1 font-mono text-[9px] text-neutral-500">
              {example.runtime}
            </span>
          </div>
          <div className="mt-5 min-h-24">
            <p className="text-[16px] font-semibold tracking-tight text-neutral-800">
              {example.name}
            </p>
            <p className="mt-2 text-xs leading-6 text-neutral-500">
              {example.detail}
            </p>
          </div>
          <div className="mt-2 border-t border-neutral-200/70 pt-4">
            <div className="mb-3 flex justify-between font-mono text-[9px] tracking-widest text-neutral-400">
              <span>INPUT</span>
              <span>RESULT</span>
            </div>
            <div className="space-y-2">
              {example.calls.map((call) => (
                <div
                  key={call.input}
                  className="grid min-h-[60px] grid-cols-[minmax(0,1fr)_14px_minmax(0,1fr)] items-center gap-2 rounded-xl border border-neutral-200/70 bg-white/80 px-3 py-3.5 text-[10px] sm:text-[11px]"
                >
                  <span className="min-w-0 text-neutral-500">{call.input}</span>
                  <Icon name="arrow" size={13} className="text-neutral-300" />
                  <span className="min-w-0 text-right font-medium text-neutral-700">
                    {call.output}
                  </span>
                </div>
              ))}
            </div>
          </div>
          <div className="mt-5 flex items-center justify-between gap-3 border-t border-neutral-200/70 pt-4">
            <span className="text-[10px] leading-5 text-neutral-500">
              {example.footer}
            </span>
            <span className="shrink-0 rounded-full border border-neutral-200 bg-white px-2.5 py-1 text-[10px] font-medium text-neutral-700">
              Free
            </span>
          </div>
        </div>
      </div>
      <figcaption className="mt-4 text-center text-[10px] text-neutral-400">
        Capability examples · ready for a fresh input
      </figcaption>
    </figure>
  );
};
export default CreatorVisual;
