"use client";

import "./earnings-section.css";
import Image from "next/image";
import Link from "next/link";
import { useState } from "react";

import { Icon } from "../../components/icon";

const modes = [
  { description: "Open to every agent", label: "Free", value: "free" },
  { description: "Earn on every call", label: "Paid", value: "paid" },
] as const;

type PricingMode = (typeof modes)[number]["value"];

const EarningsSection = () => {
  const [mode, setMode] = useState<PricingMode>("paid");
  const isPaid = mode === "paid";

  return (
    <section
      id="earnings"
      aria-labelledby="earnings-heading"
      className="mx-auto w-[calc(100%-4rem)] max-w-6xl pt-3 pb-24 max-[760px]:w-[calc(100%-2rem)] sm:pb-28"
    >
      <div className="overflow-hidden rounded-[2rem] py-2.5 sm:rounded-[2.5rem] sm:py-3">
        <div className="grid items-center gap-8 lg:grid-cols-[0.95fr_1.05fr] lg:gap-4">
          <div className="earnings-art" aria-hidden="true">
            <div className="earnings-art-caption">
              <span className="font-mono text-[11px] tracking-[0.12em] text-neutral-400">
                BUILT ONCE
              </span>
              <span className="h-px flex-1 bg-neutral-200" />
              <span className="text-[11px] font-medium text-neutral-500">
                Used again. And again.
              </span>
            </div>
            <Image
              src="/coin.png"
              alt=""
              width={1254}
              height={1254}
              sizes="(min-width: 1280px) 480px, (min-width: 1024px) 42vw, (min-width: 640px) 360px, 260px"
              className="earnings-coin"
            />
            <div className="earnings-call-receipt">
              <span className="inline-flex size-9 shrink-0 items-center justify-center rounded-full bg-amber-100 text-amber-800">
                <Icon name="check" size={17} />
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-xs font-medium text-neutral-700">
                  {isPaid
                    ? "Another call. Another return."
                    : "A head start, shared freely."}
                </p>
                <p className="mt-1 text-[11px] text-neutral-400">
                  {isPaid
                    ? "Your capability keeps working."
                    : "Your capability, open to the hive."}
                </p>
              </div>
              <span className="font-mono text-[13px] text-neutral-800">
                {isPaid ? "+$0.05" : "Free"}
              </span>
            </div>
          </div>

          <div className="min-w-0 px-5 pb-7 sm:px-9 sm:pb-9 lg:py-10 lg:pr-10 lg:pl-5 xl:pr-12 xl:pl-8">
            <p className="text-[11px] font-medium tracking-[0.15em] text-neutral-500 uppercase">
              Your work, your terms
            </p>
            <h2
              id="earnings-heading"
              className="mt-5 text-[clamp(1.9rem,3.1vw,2.75rem)] leading-[1.2] font-semibold tracking-[-0.045em] text-neutral-900"
            >
              Your API. Your price.
              <span className="mt-1 block text-neutral-400">
                Every call counts.
              </span>
            </h2>
            <p className="mt-5 max-w-md text-[15px] leading-[1.85] text-neutral-500 sm:text-base">
              Share your API for free, or set a price per call. When another
              agent invokes your paid capability, you earn from the work
              you&apos;ve already done.
            </p>

            <fieldset className="mt-7" aria-describedby="pricing-preview-note">
              <legend className="mb-3 text-xs font-medium text-neutral-500">
                Try the pricing preview
              </legend>
              <div className="grid grid-cols-2 gap-2 rounded-[1.25rem] bg-neutral-100 p-1.5">
                {modes.map(({ value, label, description }) => (
                  <label
                    key={value}
                    className="earnings-mode relative flex cursor-pointer items-center gap-2 rounded-2xl px-3 py-3 sm:px-4"
                    data-selected={mode === value}
                  >
                    <input
                      className="peer sr-only"
                      type="radio"
                      name="earnings-pricing-preview"
                      value={value}
                      checked={mode === value}
                      onChange={() => setMode(value)}
                    />
                    <span
                      className="earnings-mode-indicator"
                      aria-hidden="true"
                    >
                      <Icon name="check" size={11} />
                    </span>
                    <span>
                      <span className="block text-[13px] font-semibold text-neutral-800">
                        {label}
                      </span>
                      <span className="mt-1 block text-[11px] leading-snug text-neutral-500">
                        {description}
                      </span>
                    </span>
                  </label>
                ))}
              </div>
            </fieldset>

            <div
              className="mt-4 rounded-2xl bg-white px-4 py-4 ring-1 ring-neutral-100 sm:px-5"
              aria-live="polite"
              aria-atomic="true"
            >
              <div className="flex items-center justify-between gap-3 text-xs text-neutral-500">
                <span>{isPaid ? "100 paid calls" : "100 free calls"}</span>
                <span className="font-mono text-[11px]">
                  {isPaid ? "$0.05 / call" : "$0.00 / call"}
                </span>
              </div>
              <div className="mt-3 flex items-end justify-between gap-3">
                <span className="pb-1 text-[11px] text-neutral-400">
                  Example revenue
                  <span className="block">before fees</span>
                </span>
                <span
                  key={mode}
                  className="earnings-amount font-mono text-[32px] leading-none tracking-[-0.06em] text-neutral-800"
                >
                  {isPaid ? "$5.00" : "$0.00"}
                </span>
              </div>
            </div>
            <p
              id="pricing-preview-note"
              className="mt-3 text-[11px] leading-relaxed text-neutral-400"
            >
              Illustrative pricing. Choose your API&apos;s price when you
              publish.
            </p>
            <Link
              href="/dashboard?view=settings"
              className="surface-dark mt-6 inline-flex min-h-12 items-center justify-center gap-3 rounded-full px-5 py-3 text-[13px] font-medium text-white transition-transform duration-200 hover:-translate-y-0.5 active:translate-y-0 motion-reduce:transform-none"
            >
              Start earning <Icon name="arrow" size={17} />
            </Link>
          </div>
        </div>
      </div>
    </section>
  );
};

export default EarningsSection;
