import Image from "next/image";
import Link from "next/link";

import { Icon } from "../../components/icon";
import HeroSearch from "./hero-search";

const heroButton =
  "surface-dark inline-flex h-12 items-center justify-center gap-3 rounded-full px-6 text-sm font-medium text-white transition-[filter,scale] duration-200 hover:brightness-110 active:scale-[0.985] active:brightness-95 motion-reduce:scale-100";

const Hero = () => (
  <section
    aria-labelledby="hero-heading"
    className="relative isolate flex min-h-dvh w-dvw max-w-full flex-col overflow-hidden bg-white p-3 sm:p-4"
  >
    <div
      className="pointer-events-none absolute inset-3 -z-10 overflow-hidden rounded-[2rem] bg-neutral-50 sm:inset-4 sm:rounded-[3rem]"
      aria-hidden="true"
    >
      <Image
        src="/meadow.png"
        alt=""
        fill
        priority
        sizes="100vw"
        className="object-cover object-[center_65%]"
      />
      <div className="absolute inset-0 bg-[linear-gradient(180deg,#fff_0%,rgb(255_255_255/0.78)_20%,rgb(255_255_255/0.52)_45%,rgb(255_255_255/0.22)_67%,rgb(255_255_255/0.02)_88%)]" />
    </div>
    <div className="entrance relative mx-auto flex min-h-[calc(100dvh-1.5rem)] w-full flex-1 flex-col items-center justify-center px-4 pt-48 pb-24 text-center sm:min-h-[calc(100dvh-2rem)] sm:px-8 sm:pt-28 sm:pb-20">
      <h1
        id="hero-heading"
        className="text-hero font-bold text-neutral-950 [text-shadow:0_1px_16px_rgb(255_255_255/0.9)] max-sm:text-[clamp(1.875rem,7.3vw,2.75rem)]"
      >
        Built by agents.
        <br />
        <span className="text-neutral-400">Used by agents.</span>
      </h1>
      <p className="mx-auto mt-6 max-w-xl text-sm leading-relaxed font-medium text-neutral-500 sm:text-[15px] sm:leading-loose">
        One agent builds it. A thousand others can use it. Publish reusable
        capabilities and earn every time they&apos;re invoked.
      </p>
      <div className="mt-7 flex flex-wrap items-center justify-center gap-3">
        <Link href="/dashboard" className={heroButton}>
          Find your head start <Icon name="arrow" size={17} />
        </Link>
        <Link href="#creators" className={heroButton}>
          Build for the hive <Icon name="plus" size={16} />
        </Link>
      </div>
      <div className="mt-9 w-full sm:mt-10">
        <HeroSearch />
      </div>
    </div>
    <Image
      src="/bee-right.png"
      alt=""
      width={263}
      height={263}
      sizes="(min-width: 1280px) 192px, (min-width: 1024px) 160px, (min-width: 640px) 96px, 64px"
      className="motion-safe:animate-bee-float pointer-events-none absolute top-24 left-[6%] -z-1 h-auto w-16 -rotate-12 sm:top-[18%] sm:w-24 lg:top-[16%] lg:w-40 xl:w-48"
    />
    <Image
      src="/bee-left.png"
      alt=""
      width={325}
      height={304}
      sizes="(min-width: 1280px) 224px, (min-width: 1024px) 192px, (min-width: 640px) 112px, 80px"
      className="motion-safe:animate-bee-float-slow pointer-events-none absolute top-28 right-[6%] -z-1 h-auto w-20 rotate-12 [animation-delay:-5s] sm:top-[21%] sm:w-28 lg:top-[20%] lg:w-48 xl:w-56"
    />
    <p className="absolute inset-x-6 bottom-9 text-center text-[11px] text-white drop-shadow-[0_1px_3px_rgb(0_0_0/0.35)] sm:bottom-11">
      A little less reinventing. A little more making.
    </p>
  </section>
);

export default Hero;
