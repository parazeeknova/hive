import Link from "next/link";

import { Brand } from "../../components/brand";
import { Icon } from "../../components/icon";

const footerLink = "inline-flex items-center gap-1";

const Footer = () => (
  <footer className="mx-auto max-w-6xl px-10 pt-12 pb-5 max-[760px]:px-4">
    <div className="flex items-center justify-between pb-8 max-[760px]:flex-col max-[760px]:items-start max-[760px]:gap-6">
      <div>
        <Brand />
        <p className="mt-3 text-xs text-stone-400">
          Good work deserves a second life.
        </p>
      </div>
      <nav
        aria-label="Footer navigation"
        className="flex flex-wrap items-center gap-7 text-xs text-stone-500 max-[760px]:gap-4"
      >
        <Link href="/#capabilities" className={footerLink}>
          The network
        </Link>
        <Link href="/#creators" className={footerLink}>
          For builders
        </Link>
        <Link href="/#docs" className={footerLink}>
          The details
        </Link>
        <a
          href="https://github.com/parazeeknova/hive"
          target="_blank"
          rel="noreferrer"
          className={footerLink}
        >
          GitHub <Icon name="external" size={13} />
        </a>
      </nav>
    </div>
    <div className="flex justify-between gap-4 border-t border-stone-200 pt-5 text-xs text-stone-400 max-[760px]:flex-col max-[760px]:gap-2">
      <span>© 2026 Hive</span>
      <span>Built for the work we can do together.</span>
      <span className="inline-flex items-center gap-2">
        <span className="inline-block size-1.5 shrink-0 rounded-full bg-lime-500" />
        An early look at what’s next
      </span>
    </div>
  </footer>
);

export default Footer;
