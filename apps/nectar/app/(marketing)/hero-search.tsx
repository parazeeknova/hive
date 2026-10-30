"use client";

import { useEffect, useState } from "react";

import { Icon } from "../../components/icon";

const prompts = [
  "Audit a GitHub repo for security issues",
  "Find outdated dependencies in my repo",
  "Extract documentation from a repository",
  "Check the licenses in my codebase",
  "Map the tech stack of a repository",
  "Review my code for quality issues",
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
        className="surface-light flex h-16 items-center gap-3 rounded-full p-2 pl-5 focus-within:outline-2 focus-within:outline-offset-4 focus-within:outline-neutral-300 sm:h-18 sm:gap-4 sm:pl-6"
      >
        <Icon name="search" size={21} className="shrink-0 text-neutral-400" />
        <label className="sr-only" htmlFor="hero-capability-search">
          Search Hive capabilities
        </label>
        <input
          id="hero-capability-search"
          name="q"
          type="search"
          required
          autoComplete="off"
          aria-describedby="hero-search-hint"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          placeholder={prompt.slice(0, frame.characters)}
          className="h-full min-w-0 flex-1 border-0 bg-transparent text-base text-neutral-900 outline-none placeholder:text-neutral-400 focus-visible:outline-none motion-reduce:placeholder:text-neutral-500 sm:text-sm"
        />
        <button
          type="submit"
          aria-label="Find capabilities"
          className="surface-dark flex size-11 shrink-0 items-center justify-center rounded-full text-white transition-[filter,scale] duration-200 hover:brightness-110 active:scale-[0.985] active:brightness-95 motion-reduce:scale-100 sm:size-13"
        >
          <Icon name="arrow" size={19} />
        </button>
      </form>
      <p
        id="hero-search-hint"
        className="mt-4 text-center text-xs text-neutral-500"
      >
        Describe the job. Find a little head start.
      </p>
    </search>
  );
};

export default HeroSearch;
