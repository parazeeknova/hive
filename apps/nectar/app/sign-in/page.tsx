import Image from "next/image";
import Link from "next/link";

import { Brand } from "../../components/brand";
import { Icon } from "../../components/icon";

export const metadata = { title: "Come on in — Hive" };

const SignInPage = () => (
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
        <h1 className="mt-8 max-w-md text-4xl leading-tight font-bold tracking-tight text-stone-800 max-[760px]:text-3xl">
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
    <section className="flex items-center justify-center px-6 py-16 max-[760px]:px-5 max-[760px]:py-10">
      <div className="shadow-soft w-full max-w-md rounded-3xl border border-stone-200 bg-white p-8 max-[760px]:p-6">
        <span className="text-xs font-bold tracking-widest text-stone-400 uppercase">
          Good to see you here
        </span>
        <h2 className="mt-3 text-2xl font-bold tracking-tight">Come on in.</h2>
        <p className="mt-4 text-sm leading-loose text-stone-500">
          We’re still putting the finishing touches on account sign-in. In the
          meantime, the preview workspace is open for a look around.
        </p>
        <Link
          href="/dashboard"
          className="mt-6 inline-flex min-h-12 items-center justify-center gap-3 rounded-xl bg-stone-800 px-6 py-3 text-sm font-bold text-white shadow-sm transition-[transform,background] duration-200 hover:-translate-y-0.5 hover:bg-stone-700 active:translate-y-0"
        >
          Explore the preview <Icon name="arrow" size={17} />
        </Link>
        <div className="mt-6 flex items-start gap-2 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs leading-relaxed text-amber-800 [&>svg]:mt-0.5 [&>svg]:shrink-0">
          <Icon name="help" size={16} />
          No account needed. Example listings, local drafts, and sample
          responses. No live payments.
        </div>
        <Link
          href="/"
          className="mt-6 inline-flex items-center gap-2.5 text-xs font-bold text-lime-700"
        >
          ← Back to the meadow
        </Link>
      </div>
    </section>
  </main>
);

export default SignInPage;
