import { MarkGithubIcon } from "@primer/octicons-react";
import Image from "next/image";
import Link from "next/link";
import { redirect } from "next/navigation";

import { Brand } from "../../components/brand";
import { dashboardReturnPath } from "../../lib/auth-redirect";
import { privateMetadata } from "../../lib/seo";
import { session } from "../../lib/server/session";

export const metadata = { ...privateMetadata, title: "Sign in — Hive" };

// Landing tokens (see app/(marketing)/landing.css): paper, ink, honey, earth, muted.
const radius = "rounded-[clamp(14px,1.5vw,22px)]";
const focus =
  "focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-[#24251f]";

const SignInPage = async ({
  searchParams,
}: {
  searchParams: Promise<{ next?: string; error?: string }>;
}) => {
  const params = await searchParams;
  const next = dashboardReturnPath(params.next);
  if (await session()) {
    redirect(next);
  }
  return (
    <div className="font-body flex min-h-dvh flex-col gap-[clamp(10px,1.3vw,18px)] overflow-x-clip bg-[#fffefb] p-[clamp(10px,1.3vw,18px)] text-[17px] leading-[1.55] text-[#24251f]">
      <header className="flex items-center justify-between gap-4 px-[clamp(8px,1.5vw,20px)] pt-1">
        <Brand markClassName="size-8" wordmarkClassName="text-3xl" />
        <Link
          href="/"
          className={`inline-flex min-h-[42px] items-center gap-2 rounded-full border-[1.5px] border-dashed border-current px-[18px] text-[15px] leading-none font-medium whitespace-nowrap transition-colors duration-300 hover:border-solid hover:bg-[#24251f] hover:text-[#f8cf60] ${focus}`}
        >
          <span aria-hidden="true">←</span> Back home
        </Link>
      </header>
      <main
        id="main"
        className="grid flex-1 grid-cols-[minmax(0,7fr)_minmax(0,5fr)] gap-[clamp(10px,1.3vw,18px)] max-[860px]:grid-cols-1 max-[860px]:content-start"
      >
        <section
          aria-label="Hive meadow"
          className={`relative isolate min-h-[560px] overflow-hidden bg-[#3d4535] ${radius} max-[860px]:min-h-[clamp(220px,38svh,360px)]`}
        >
          <Image
            src="/landing/hive-meadow.webp"
            alt="A honeybee resting on a chamomile flower in a quiet meadow"
            fill
            priority
            sizes="(max-width: 860px) 100vw, 58vw"
            className="-z-1 object-cover object-[72%_60%]"
          />
          <p className="absolute bottom-0 left-0 max-w-[22em] rounded-tr-[clamp(14px,1.5vw,22px)] bg-[#fffefb] px-6 pt-4 pb-2 text-[15px] leading-[1.45] text-[#727268] max-[860px]:hidden">
            A little less reinventing. A little more making.
          </p>
        </section>
        <section
          aria-labelledby="sign-in-title"
          className={`relative isolate flex flex-col justify-between gap-12 overflow-hidden bg-[#f8cf60] p-[clamp(28px,3.4vw,48px)] ${radius} max-[860px]:gap-10 max-[520px]:p-6`}
        >
          <Image
            src="/bee-left.png"
            alt=""
            width={325}
            height={304}
            className="motion-safe:animate-bee-float pointer-events-none absolute -top-2 right-[clamp(16px,3vw,40px)] -z-1 h-auto w-[clamp(72px,8vw,112px)] rotate-6 max-[520px]:w-16"
          />
          <div>
            <p className="text-[14px] leading-[1.35] font-medium tracking-[0.02em]">
              COME ON IN.
            </p>
            <h1
              id="sign-in-title"
              className="mt-6 max-w-[11em] pr-16 text-[clamp(32px,3.6vw,52px)] leading-[1.04] font-normal tracking-[-0.032em] text-balance max-[520px]:pr-12"
            >
              Your head start is waiting.
            </h1>
            <p className="mt-5 max-w-[26em] text-[clamp(17px,1.4vw,20px)] leading-[1.42] tracking-[-0.008em]">
              Sign in with GitHub to find tools for your agent, run them, and
              publish your own.
            </p>
          </div>
          <div className="grid gap-7">
            {params.error && (
              <p
                role="alert"
                className="rounded-xl border-[1.5px] border-[#24251f] bg-[#fffefb] px-4 py-3 text-[15px] leading-snug"
              >
                GitHub sign-in didn’t go through. Try once more; your search is
                saved.
              </p>
            )}
            <Link
              href={`/api/auth/github?${new URLSearchParams({ next })}`}
              prefetch={false}
              className={`group inline-flex min-h-[56px] w-full items-center justify-center gap-3 rounded-full border border-transparent px-[26px] text-[16px] leading-none font-medium text-white shadow-[inset_0_1px_2px_#ffffff33,inset_0_-2px_4px_#0006,inset_1px_0_3px_#ffffff0b] transition-[filter,transform] duration-200 [background:radial-gradient(ellipse_at_50%_0%,#ffffff0c,transparent_70%)_padding-box,linear-gradient(180deg,#333_0%,#222_24%,#151515_58%,#101010_80%,#1b1b1b_100%)_padding-box,linear-gradient(180deg,#505050,#171717_65%,#080808)_border-box] hover:brightness-[1.12] active:translate-y-px active:scale-[0.985] active:brightness-95 motion-reduce:active:transform-none ${focus}`}
            >
              <MarkGithubIcon size={18} />
              <span>Continue with GitHub</span>
              <span
                aria-hidden="true"
                className="transition-transform duration-300 group-hover:translate-x-1 motion-reduce:transition-none"
              >
                →
              </span>
            </Link>
            <p className="max-w-[34em] text-[15px] leading-[1.5]">
              New to Hive? Signing in creates your account. Hive only reads your
              public GitHub profile, and you’ll come back right where you left
              off.
            </p>
            <p className="border-t-[1.5px] border-dashed border-[#24251f]/40 pt-5 text-[14px] leading-[1.45]">
              Connecting an agent instead? It uses an API key you create after
              signing in.{" "}
              <Link
                href="/docs#docs-credentials"
                className={`font-medium underline decoration-[1.5px] underline-offset-4 ${focus}`}
              >
                Read the setup guide
              </Link>
            </p>
          </div>
        </section>
      </main>
    </div>
  );
};

export default SignInPage;
