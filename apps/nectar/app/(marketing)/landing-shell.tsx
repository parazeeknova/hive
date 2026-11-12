"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import { Brand } from "../../components/brand";

import "./landing.css";

// Real destinations across the site, not only landing anchors.
const navigation = [
  { href: "/#workflow", label: "How it works" },
  { href: "/dashboard?view=discover", label: "Explore tools" },
  { href: "/docs", label: "Docs" },
  { href: "/dashboard?view=settings", label: "Publish" },
];
export const LandingButton = ({
  children,
  href,
  outline = false,
  className = "",
}: {
  children: ReactNode;
  href: string;
  outline?: boolean;
  className?: string;
}) => (
  <Link
    href={href}
    className={`${outline ? "hive-btn-2" : "hive-btn"} ${className}`}
  >
    {outline && <span className="hive-btn-2__icon" aria-hidden="true" />}
    <span className={outline ? "hive-btn-2__label" : "hive-btn__label"}>
      {children}
    </span>
  </Link>
);
const LandingNavigation = ({ username }: { username: string | null }) => {
  const pathname = usePathname();
  const [open, setOpen] = useState(false);
  const header = useRef<HTMLElement>(null);
  const toggle = useRef<HTMLButtonElement>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    let last = window.scrollY;
    const update = () => {
      const y = window.scrollY;
      if (header.current) {
        if (y < 80) {
          header.current.dataset.state = "top";
        } else if (y > last && y > 200) {
          header.current.dataset.state = "hidden";
        } else {
          header.current.dataset.state = "float";
        }
      }
      last = y;
    };
    window.addEventListener("scroll", update, { passive: true });
    return () => window.removeEventListener("scroll", update);
  }, []);
  useEffect(() => {
    if (!open) {
      return;
    }
    const previous = document.body.style.overflow;
    const trigger = toggle.current;
    document.body.style.overflow = "hidden";
    const controls = dialog.current?.querySelectorAll<HTMLElement>("a, button");
    controls?.[0]?.focus();
    const keyboard = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
      if (event.key !== "Tab" || !controls?.length) {
        return;
      }
      const [first] = controls;
      const last = [...controls].at(-1);
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    };
    document.addEventListener("keydown", keyboard);
    return () => {
      document.body.style.overflow = previous;
      document.removeEventListener("keydown", keyboard);
      trigger?.focus();
    };
  }, [open]);
  return (
    <header id="hive-navigation" data-state="top" ref={header}>
      <div className="nav__logo">
        <Brand markClassName="size-8" wordmarkClassName="text-3xl" />
      </div>
      <nav className="nav__links" aria-label="Main navigation">
        {navigation.map((item) => (
          <Link
            className="hive-link-2"
            key={item.href}
            href={item.href}
            aria-current={pathname === item.href ? "page" : undefined}
          >
            {item.label}
          </Link>
        ))}
      </nav>
      <LandingButton className="nav__cta" href="/dashboard" outline>
        {username ? `@${username}` : "Meet the hive"}
      </LandingButton>
      <button
        ref={toggle}
        className="hive-btn-2 nav__menu"
        type="button"
        aria-expanded={open}
        aria-controls="hive-mobile-menu"
        onClick={() => setOpen(true)}
      >
        Menu <span className="hive-menu-mark" aria-hidden="true" />
      </button>
      <dialog
        open={open}
        id="hive-mobile-menu"
        className="menu"
        hidden={!open}
        ref={dialog}
        aria-modal={open || undefined}
        aria-label="Navigation"
        data-lenis-prevent
      >
        <div className="menu__top">
          <Brand />
          <button
            className="hive-round"
            type="button"
            data-icon="close"
            aria-label="Close navigation"
            onClick={() => setOpen(false)}
          >
            <span className="hive-round__icon" aria-hidden="true" />
          </button>
        </div>
        <nav className="menu__links" aria-label="Mobile navigation">
          {[...navigation, { href: "/#questions", label: "Questions" }].map(
            (item) => (
              <Link
                key={item.href}
                href={item.href}
                onClick={() => setOpen(false)}
              >
                {item.label}
              </Link>
            )
          )}
        </nav>
        <Link
          className="hive-btn"
          href="/dashboard"
          onClick={() => setOpen(false)}
        >
          {username ? "Open your workspace" : "Get started with GitHub"}
        </Link>
      </dialog>
    </header>
  );
};
const LandingFooter = () => (
  <footer id="hive-footer">
    <div className="footer__panel hive-plot hive-surface-4">
      <div className="footer__cols">
        <div className="footer__about">
          <p className="hive-text-5">
            A shared toolbox for AI agents. Discover what works. Build on it.
            Give your best work a second life.
          </p>
          <LandingButton href="/dashboard" outline>
            Find your head start
          </LandingButton>
        </div>
        <nav className="footer__nav" aria-label="Footer navigation">
          {[...navigation, { href: "/#questions", label: "Questions" }].map(
            (item) => (
              <Link className="hive-link-2" key={item.href} href={item.href}>
                {item.label}
              </Link>
            )
          )}
        </nav>
        <div className="footer__community">
          <span className="hive-text-4">
            Made for the work we can do together.
          </span>
          <a
            className="hive-link-2"
            href="https://github.com/parazeeknova/hive"
            target="_blank"
            rel="noreferrer"
          >
            Explore the source{" "}
            <span className="hive-inline-arrow" aria-hidden="true" />
          </a>
          <Link className="hive-link-2" href="/sign-in">
            Sign in with GitHub{" "}
            <span className="hive-inline-arrow" aria-hidden="true" />
          </Link>
          <a
            className="hive-link-2"
            href="https://itssingularity.com"
            target="_blank"
            rel="noreferrer"
          >
            Singularity Works{" "}
            <span className="hive-inline-arrow" aria-hidden="true" />
          </a>
          <Link className="hive-link-2" href="/docs#docs-credentials">
            Create an API key{" "}
            <span className="hive-inline-arrow" aria-hidden="true" />
          </Link>
        </div>
      </div>
      <p className="footer__word" aria-hidden="true">
        hive<span>.</span>
      </p>
      <div className="footer__small hive-text-4">
        <span>
          © 2026 Hive, by{" "}
          <a
            className="hive-link-2"
            href="https://itssingularity.com"
            target="_blank"
            rel="noreferrer"
          >
            Singularity Works
          </a>{" "}
          · Good work deserves a second life.
        </span>
        <a className="hive-link-2" href="#main">
          Back to the top{" "}
          <span
            className="hive-inline-arrow hive-inline-arrow--up"
            aria-hidden="true"
          />
        </a>
      </div>
    </div>
  </footer>
);
const MarketingShell = ({
  children,
  username,
}: {
  children: ReactNode;
  username: string | null;
}) => (
  <div className="hive-landing">
    <a
      href="#main"
      className="fixed -top-20 left-5 z-50 rounded-xl bg-stone-800 px-5 py-3 text-white focus:top-4"
    >
      Skip to content
    </a>
    <LandingNavigation username={username} />
    <main id="main">{children}</main>
    <LandingFooter />
  </div>
);
export default MarketingShell;
