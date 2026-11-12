"use client";
import Image from "next/image";
import Link from "next/link";
import type { KeyboardEvent } from "react";

import { Brand } from "../../components/brand";
import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";

export interface WorkspaceTab<Id extends string> {
  id: Id;
  label: string;
  icon: IconName;
}

/**
 * Browser-style workspace tabs. The active tab shares the page background so it
 * reads as attached to the content below; inactive tabs sit on the header.
 */
export const WorkspaceTabs = <Id extends string>({
  tabs,
  active,
  onSelect,
  account,
  avatar,
  plan,
  balance,
  onAccount,
}: {
  tabs: WorkspaceTab<Id>[];
  active: Id;
  onSelect: (id: Id) => void;
  account: string;
  avatar: string | undefined;
  plan: string | undefined;
  balance: string;
  onAccount: () => void;
}) => {
  const keyboard = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") {
      return;
    }
    event.preventDefault();
    const index = tabs.findIndex((tab) => tab.id === active);
    const step = event.key === "ArrowRight" ? 1 : -1;
    const next = tabs.at((index + step) % tabs.length);
    if (next) {
      onSelect(next.id);
      requestAnimationFrame(() =>
        document.querySelector<HTMLButtonElement>(`#ws-tab-${next.id}`)?.focus()
      );
    }
  };
  return (
    <header className="ws-tabbar sticky top-0 z-30 bg-[#eeeee7]">
      <div className="flex h-16 items-end gap-[clamp(10px,2vw,28px)] px-[clamp(12px,1.6vw,22px)] max-[760px]:h-auto max-[760px]:flex-wrap max-[760px]:gap-0 max-[760px]:px-0">
        <div className="flex h-16 shrink-0 items-center gap-2.5 max-[760px]:order-1 max-[760px]:h-14 max-[760px]:px-4">
          <Brand small />
          <span className="rounded-full border-[1.5px] border-dashed border-[#24251f]/40 px-2 py-0.5 text-[10px] font-semibold tracking-[0.08em] text-[#3d4535]">
            BETA
          </span>
        </div>
        <div
          role="tablist"
          aria-label="Workspace"
          className="flex min-w-0 flex-1 [scrollbar-width:none] items-end gap-1 self-stretch overflow-x-auto max-[760px]:order-3 max-[760px]:basis-full max-[760px]:px-2 [&::-webkit-scrollbar]:hidden"
        >
          {tabs.map((tab) => {
            const selected = tab.id === active;
            return (
              <button
                key={tab.id}
                id={`ws-tab-${tab.id}`}
                type="button"
                role="tab"
                aria-selected={selected}
                aria-controls="workspace-content"
                tabIndex={selected ? 0 : -1}
                onClick={() => onSelect(tab.id)}
                onKeyDown={keyboard}
                className={`relative flex h-12 shrink-0 cursor-pointer items-center gap-2 rounded-t-[14px] px-[18px] text-[14px] leading-none font-medium whitespace-nowrap transition-colors focus-visible:outline-2 focus-visible:-outline-offset-4 focus-visible:outline-[#24251f] max-[760px]:h-11 max-[760px]:px-3.5 max-[760px]:text-[13px] ${
                  selected
                    ? "bg-[#fffefb] text-[#24251f] before:absolute before:bottom-0 before:-left-3.5 before:size-3.5 before:bg-[radial-gradient(circle_at_0_0,transparent_13.5px,#fffefb_14px)] after:absolute after:-right-3.5 after:bottom-0 after:size-3.5 after:bg-[radial-gradient(circle_at_100%_0,transparent_13.5px,#fffefb_14px)]"
                    : "text-[#3d4535]/80 hover:bg-[#fffefb]/55 hover:text-[#24251f]"
                }`}
              >
                <Icon name={tab.icon} size={16} />
                {tab.label}
                {tab.id === "discover" && (
                  <kbd className="ml-1 rounded-md border border-[#24251f]/15 px-1.5 py-0.5 font-sans text-[10px] text-[#727268] max-[760px]:hidden">
                    ⌘K
                  </kbd>
                )}
              </button>
            );
          })}
        </div>
        <div className="ml-auto flex h-16 shrink-0 items-center gap-2 max-[760px]:order-2 max-[760px]:h-14 max-[760px]:px-3">
          <Link
            href="/docs"
            className="inline-flex min-h-9 items-center gap-1.5 rounded-full px-3 text-[13px] font-medium text-[#3d4535] transition-colors hover:bg-[#fffefb] max-[1100px]:hidden"
          >
            <Icon name="book" size={15} />
            Docs
          </Link>
          <button
            type="button"
            onClick={onAccount}
            title="Agent balance"
            className="inline-flex min-h-9 cursor-pointer items-center gap-2 rounded-full border-[1.5px] border-dashed border-[#24251f]/40 px-3 text-[13px] font-medium text-[#24251f] transition-colors hover:border-solid hover:border-[#24251f] hover:bg-[#fffefb] max-[460px]:hidden"
          >
            <Icon name="wallet" size={15} />
            <span className="tabular-nums">{balance}</span>
            {plan && (
              <span className="rounded-full bg-[#f8cf60] px-2 py-0.5 text-[11px] capitalize">
                {plan}
              </span>
            )}
          </button>
          <button
            type="button"
            onClick={onAccount}
            aria-label={`Account settings for ${account}`}
            className="inline-flex min-h-9 cursor-pointer items-center gap-2 rounded-full bg-[#fffefb] py-1 pr-3 pl-1 text-[13px] font-medium text-[#24251f] transition-colors hover:bg-[#fffefb]/70"
          >
            <span className="grid size-7 shrink-0 place-items-center overflow-hidden rounded-full bg-[#24251f] text-[11px] text-[#fffefb]">
              {avatar ? (
                <Image
                  unoptimized
                  src={avatar}
                  alt=""
                  width={28}
                  height={28}
                  referrerPolicy="no-referrer"
                  className="size-7 object-cover"
                />
              ) : (
                <span aria-hidden="true">H</span>
              )}
            </span>
            <span className="max-w-[12ch] truncate max-[560px]:hidden">
              {account}
            </span>
          </button>
        </div>
      </div>
    </header>
  );
};
