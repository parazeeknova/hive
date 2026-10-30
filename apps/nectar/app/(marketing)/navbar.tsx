"use client";

import Link from "next/link";
import { useState } from "react";

import { Brand } from "../../components/brand";
import { Icon } from "../../components/icon";

const links = [
  { href: "/#capabilities", label: "The network" },
  { href: "/#how-it-works", label: "How it works" },
  { href: "/#creators", label: "For builders" },
  { href: "/docs", label: "Developers" },
];

const navLink =
  "inline-flex h-10 items-center justify-center rounded-full px-2 text-[13px] leading-none font-medium whitespace-nowrap text-neutral-500 transition-colors hover:text-black";

const Navbar = ({ username }: { username: string | null }) => {
  const [open, setOpen] = useState(false);
  return (
    <header className="fixed top-6 left-0 z-40 w-full px-8 font-medium max-[760px]:top-4 max-[760px]:px-4">
      <nav
        aria-label="Main navigation"
        className="surface-light surface-glass mx-auto grid max-w-6xl grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-4 rounded-full border-2 border-neutral-300 px-4 py-1.5 backdrop-blur-xl backdrop-saturate-125 max-[1000px]:grid-cols-[auto_minmax(0,1fr)] max-[760px]:gap-2 max-[760px]:px-3"
      >
        <div className="flex h-10 items-center justify-self-start">
          {/* Offset the wordmark's font box to center its visible glyphs. */}
          <Brand
            markClassName="size-7 sm:size-[30px]"
            wordmarkClassName="translate-y-[0.13em] text-[26px] sm:text-[28px]"
          />
        </div>
        <div className="flex items-center justify-center gap-2 max-[1000px]:hidden">
          {links.map((link) => (
            <Link key={link.href} href={link.href} className={navLink}>
              {link.label}
            </Link>
          ))}
        </div>
        <div className="flex items-center justify-end gap-2 justify-self-end">
          {!username && (
            <Link className={`${navLink} max-[760px]:hidden`} href="/sign-in">
              Sign in
            </Link>
          )}
          <Link
            className="surface-dark inline-flex h-10 items-center justify-center gap-2 rounded-full px-4 text-[13px] leading-none font-medium whitespace-nowrap text-white transition-[filter,scale] duration-200 hover:brightness-110 active:scale-[0.985] active:brightness-95 motion-reduce:scale-100 motion-reduce:transform-none max-[760px]:px-3 max-[760px]:text-xs"
            href="/dashboard"
            aria-label={username ? `Open dashboard for ${username}` : undefined}
          >
            <span className="max-w-40 truncate max-[760px]:max-w-24">
              {username ? `@${username}` : "Meet the hive"}
            </span>
            <Icon name="arrow" size={16} />
          </Link>
          <button
            className="surface-light hidden size-10 items-center justify-center rounded-full text-neutral-600 hover:text-black max-[1000px]:flex"
            type="button"
            aria-label={open ? "Close navigation" : "Open navigation"}
            aria-controls="mobile-navigation"
            aria-expanded={open}
            onClick={() => setOpen(!open)}
          >
            <Icon name={open ? "close" : "menu"} />
          </button>
        </div>
      </nav>
      {open && (
        <nav
          aria-label="Mobile navigation"
          id="mobile-navigation"
          className="surface-light surface-glass mx-auto mt-3 hidden max-w-6xl rounded-3xl border-2 border-neutral-300 p-2 backdrop-blur-xl backdrop-saturate-125 max-[1000px]:grid"
        >
          {links.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              onClick={() => setOpen(false)}
              className="flex min-h-11 items-center rounded-full px-4 text-sm font-medium text-neutral-600 hover:bg-neutral-100"
            >
              {link.label}
            </Link>
          ))}
          <Link
            href={username ? "/dashboard" : "/sign-in"}
            onClick={() => setOpen(false)}
            className="flex min-h-11 items-center rounded-full px-4 text-sm font-medium text-neutral-600 hover:bg-neutral-100"
          >
            {username ? `@${username} · Dashboard` : "Sign in"}
          </Link>
        </nav>
      )}
    </header>
  );
};

export default Navbar;
