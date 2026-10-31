import { ArrowRightIcon, MarkGithubIcon } from "@primer/octicons-react";
import Image from "next/image";
import Link from "next/link";
import { redirect } from "next/navigation";

import { Brand } from "../../components/brand";
import { dashboardReturnPath } from "../../lib/auth-redirect";
import { privateMetadata } from "../../lib/seo";
import { session } from "../../lib/server/session";

export const metadata = { ...privateMetadata, title: "Come on in — Hive" };

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
    <main className="grid min-h-dvh grid-cols-2 bg-stone-50 max-[760px]:grid-cols-1">
      <section className="relative isolate flex flex-col justify-center overflow-hidden px-12 py-16 max-[760px]:min-h-64 max-[760px]:px-6 max-[760px]:py-10">
        <Image
          src="/meadow.png"
          alt=""
          fill
          priority
          sizes="(min-width: 760px) 50vw, 100vw"
          className="-z-1 object-cover opacity-60"
        />
        <div className="relative z-2">
          <Brand />
          <h1 className="font-display mt-8 max-w-md text-4xl leading-tight font-bold tracking-tight text-stone-800 max-[760px]:text-3xl">
            A little less building.
            <br />A lot more doing.
          </h1>
          <p className="mt-4 max-w-sm text-sm leading-loose text-stone-600">
            Your next useful tool might already be in the hive.
          </p>
        </div>
        <Image
          src="/bee-left.png"
          alt=""
          width={325}
          height={304}
          className="absolute right-4 bottom-4 z-2 h-auto w-40 rotate-6 drop-shadow-md max-[760px]:hidden"
        />
      </section>
      <section className="flex min-h-dvh flex-col items-center justify-center bg-white px-8 py-16 text-[#e6edf3] max-[760px]:min-h-0 max-[760px]:px-5 max-[760px]:py-14">
        <div className="w-full max-w-sm rounded-3xl border border-[#30363d] bg-[#0d1117] p-8 shadow-[0_24px_64px_-24px_#0d11174d] max-[760px]:p-6">
          <h2 className="font-display text-2xl font-bold tracking-tight text-white">
            Come on in.
          </h2>
          <p className="mt-3 text-sm leading-7 text-[#8b949e]">
            Sign in or create your Hive account with GitHub. Your next useful
            capability is waiting.
          </p>
          {params.error && (
            <p role="alert" className="mt-4 text-sm text-red-300">
              GitHub sign-in failed. Try again to continue your search.
            </p>
          )}
          <Link
            href={`/api/auth/github?${new URLSearchParams({ next })}`}
            prefetch={false}
            className="surface-dark mt-8 inline-flex h-10 w-full items-center justify-center gap-2 rounded-full px-4 text-[13px] leading-none font-medium whitespace-nowrap text-white shadow-[inset_0_1px_2px_rgb(255_255_255_/_0.2),_inset_0_-2px_4px_rgb(0_0_0_/_0.4),_inset_1px_0_3px_rgb(255_255_255_/_0.045)] transition-[filter,scale] duration-200 [background:radial-gradient(ellipse_at_50%_0%,_#ffffff0c,_transparent_70%)_padding-box,_linear-gradient(_180deg,_#333_0%,_#222_24%,_#151515_58%,_#101010_80%,_#1b1b1b_100%_)_padding-box,_linear-gradient(180deg,_#505050,_#171717_65%,_#080808)_border-box] [border:1px_solid_transparent] hover:brightness-110 focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-white active:scale-[0.985] active:brightness-95 motion-reduce:scale-100 motion-reduce:transform-none"
          >
            <MarkGithubIcon size={16} />
            <span>Continue with GitHub</span>
            <ArrowRightIcon size={16} />
          </Link>
        </div>
        <Link
          href="/"
          className="mt-7 inline-flex items-center gap-2 text-xs font-medium text-neutral-500 transition-colors hover:text-black focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-neutral-800"
        >
          <span aria-hidden="true">←</span> Back to the meadow
        </Link>
      </section>
    </main>
  );
};

export default SignInPage;
