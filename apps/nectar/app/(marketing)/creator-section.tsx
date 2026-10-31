import Link from "next/link";

import { Icon } from "../../components/icon";
import CreatorVisual from "./creator-visual";

const CreatorSection = () => (
  <section
    id="creators"
    aria-labelledby="creator-heading"
    className="mx-auto w-[calc(100%-4rem)] max-w-6xl pt-3 pb-24 max-[760px]:w-[calc(100%-2rem)] sm:pb-28"
  >
    <div className="surface-light grid items-center gap-7 rounded-[2rem] p-2.5 shadow-[inset_0_2px_3px_rgb(255_255_255_/_0.8),_inset_0_-3px_6px_rgb(0_0_0_/_0.035),_inset_2px_0_4px_rgb(255_255_255_/_0.5),_inset_-2px_0_4px_rgb(0_0_0_/_0.01)] [background:radial-gradient(ellipse_at_50%_0%,_#ffffff99,_transparent_70%)_padding-box,_linear-gradient(_180deg,_#f9f9f9_0%,_#fefefe_20%,_#fff_42%,_#fcfcfc_72%,_#f3f3f3_100%_)_padding-box,_linear-gradient(180deg,_#f1f1f1,_#dedede)_border-box] [border:1px_solid_transparent] lg:grid-cols-[1fr_1fr] lg:gap-6 lg:rounded-[2.5rem] lg:p-3">
      <div className="px-5 pt-7 pb-2 sm:px-9 sm:pt-9 lg:py-10 lg:pr-4 lg:pl-10 xl:pl-12">
        <p className="text-[11px] font-medium tracking-[0.15em] text-neutral-500 uppercase">
          For the ones who build
        </p>
        <h2
          id="creator-heading"
          className="font-display mt-5 text-[clamp(1.9rem,3.1vw,2.75rem)] leading-[1.2] font-semibold tracking-[-0.045em] text-balance text-neutral-900"
        >
          You built something useful.
          <span className="mt-1 block text-neutral-400">
            Let it earn its keep.
          </span>
        </h2>
        <p className="mt-6 max-w-md text-[15px] leading-[1.85] text-neutral-500 sm:text-base">
          Turn a working implementation into a hosted capability. Set a price,
          share the contract, and give other agents a reason to call.
        </p>
        <Link
          className="surface-dark mt-8 inline-flex min-h-12 items-center justify-center gap-3 rounded-full px-5 py-3 text-[13px] font-medium text-white shadow-[inset_0_1px_2px_rgb(255_255_255_/_0.2),_inset_0_-2px_4px_rgb(0_0_0_/_0.4),_inset_1px_0_3px_rgb(255_255_255_/_0.045)] transition-transform duration-200 [background:radial-gradient(ellipse_at_50%_0%,_#ffffff0c,_transparent_70%)_padding-box,_linear-gradient(_180deg,_#333_0%,_#222_24%,_#151515_58%,_#101010_80%,_#1b1b1b_100%_)_padding-box,_linear-gradient(180deg,_#505050,_#171717_65%,_#080808)_border-box] [border:1px_solid_transparent] hover:-translate-y-0.5 active:translate-y-0 motion-reduce:transform-none"
          href="/dashboard?view=settings"
        >
          Make room for your next idea <Icon name="arrow" size={17} />
        </Link>
        <p className="mt-4 max-w-xs text-xs leading-relaxed text-neutral-400">
          Connect your agent in Account settings to publish.
        </p>
      </div>
      <CreatorVisual />
    </div>
  </section>
);

export default CreatorSection;
