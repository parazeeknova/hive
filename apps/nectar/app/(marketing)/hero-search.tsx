"use client";

import { useEffect, useState } from "react";

import { Icon } from "../../components/icon";

const prompts = [
  "Repository stack",
  "Dependency inventory",
  "JSON echo",
  "Repository workflow",
] as const;

interface TypingFrame {
  characters: number;
  phase: "hold" | "delete" | "type";
  prompt: number;
}

const HeroSearch = () => {
  const [query, setQuery] = useState("");
  const [focused, setFocused] = useState(false);
  const [frame, setFrame] = useState<TypingFrame>({
    characters: prompts[0].length,
    phase: "hold",
    prompt: 0,
  });

  useEffect(() => {
    if (focused || query) {
      return;
    }
    const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
    let timer: number | undefined;
    const advance = () => {
      if (frame.phase === "hold") {
        setFrame({ ...frame, phase: "delete" });
        return;
      }
      if (frame.phase === "delete") {
        if (frame.characters > 0) {
          setFrame({ ...frame, characters: frame.characters - 1 });
        } else {
          const offset = 1 + Math.floor(Math.random() * (prompts.length - 1));
          setFrame({
            characters: 0,
            phase: "type",
            prompt: (frame.prompt + offset) % prompts.length,
          });
        }
        return;
      }
      const prompt = prompts[frame.prompt] ?? prompts[0];
      const characters = Math.min(frame.characters + 1, prompt.length);
      setFrame({
        ...frame,
        characters,
        phase: characters === prompt.length ? "hold" : "type",
      });
    };
    const schedule = () => {
      window.clearTimeout(timer);
      if (motion.matches) {
        return;
      }
      let delay = 65;
      if (frame.phase === "hold") {
        delay = 2200;
      }
      if (frame.phase === "delete") {
        delay = frame.characters === 0 ? 300 : 24;
      }
      timer = window.setTimeout(advance, delay);
    };
    schedule();
    motion.addEventListener("change", schedule);
    return () => {
      window.clearTimeout(timer);
      motion.removeEventListener("change", schedule);
    };
  }, [focused, frame, query]);

  const prompt = prompts[frame.prompt] ?? prompts[0];

  return (
    <search className="mx-auto w-full max-w-2xl">
      <form
        action="/dashboard"
        method="get"
        className="surface-light flex h-16 items-center gap-3 rounded-full p-2 pl-5 shadow-[inset_0_2px_3px_rgb(255_255_255_/_0.8),_inset_0_-3px_6px_rgb(0_0_0_/_0.035),_inset_2px_0_4px_rgb(255_255_255_/_0.5),_inset_-2px_0_4px_rgb(0_0_0_/_0.01)] [background:radial-gradient(ellipse_at_50%_0%,_#ffffff99,_transparent_70%)_padding-box,_linear-gradient(_180deg,_#f9f9f9_0%,_#fefefe_20%,_#fff_42%,_#fcfcfc_72%,_#f3f3f3_100%_)_padding-box,_linear-gradient(180deg,_#f1f1f1,_#dedede)_border-box] [border:1px_solid_transparent] focus-within:outline-2 focus-within:outline-offset-4 focus-within:outline-neutral-300 sm:h-18 sm:gap-4 sm:pl-6"
      >
        <input type="hidden" name="view" value="discover" />
        <Icon name="search" size={21} className="shrink-0 text-neutral-400" />
        <label className="sr-only" htmlFor="hero-capability-search">
          Search Hive capabilities
        </label>
        <input
          id="hero-capability-search"
          name="q"
          type="search"
          required
          maxLength={200}
          autoComplete="off"
          aria-describedby="hero-search-hint"
          value={query}
          onChange={(event) => {
            if (new TextEncoder().encode(event.target.value).length <= 200) {
              setQuery(event.target.value);
            }
          }}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          placeholder={prompt.slice(0, frame.characters)}
          className="h-full min-w-0 flex-1 border-0 bg-transparent text-base text-neutral-900 outline-none placeholder:text-neutral-400 focus-visible:outline-none motion-reduce:placeholder:text-neutral-500 sm:text-sm"
        />
        <button
          type="submit"
          aria-label="Find capabilities"
          className="surface-dark flex size-11 shrink-0 cursor-pointer items-center justify-center rounded-full text-white shadow-[inset_0_1px_2px_rgb(255_255_255_/_0.2),_inset_0_-2px_4px_rgb(0_0_0_/_0.4),_inset_1px_0_3px_rgb(255_255_255_/_0.045)] transition-[filter,scale] duration-200 [background:radial-gradient(ellipse_at_50%_0%,_#ffffff0c,_transparent_70%)_padding-box,_linear-gradient(_180deg,_#333_0%,_#222_24%,_#151515_58%,_#101010_80%,_#1b1b1b_100%_)_padding-box,_linear-gradient(180deg,_#505050,_#171717_65%,_#080808)_border-box] [border:1px_solid_transparent] hover:brightness-110 active:scale-[0.985] active:brightness-95 disabled:cursor-not-allowed disabled:opacity-[0.45] motion-reduce:scale-100 sm:size-13"
        >
          <Icon name="arrow" size={19} />
        </button>
      </form>
      <p
        id="hero-search-hint"
        className="mt-4 text-center text-xs text-neutral-500"
      >
        Search capabilities by name or description.
      </p>
    </search>
  );
};

export default HeroSearch;
