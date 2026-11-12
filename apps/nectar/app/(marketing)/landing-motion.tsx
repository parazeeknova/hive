"use client";
import { useEffect } from "react";

const loadMotion = () =>
  Promise.all([
    import("gsap"),
    import("gsap/ScrollTrigger"),
    import("gsap/SplitText"),
    import("lenis"),
  ]);
const centre = (card: HTMLElement) => ({
  x: card.offsetLeft + card.offsetWidth / 2,
  y: card.offsetTop + card.offsetHeight / 2,
});
const LandingMotion = () => {
  useEffect(() => {
    let disposed = false;
    let teardown: (() => void) | undefined;
    const setup = async () => {
      const [{ gsap }, { ScrollTrigger }, { SplitText }, { default: Lenis }] =
        await loadMotion();
      // The reference measures and splits text only after its fonts are ready.
      await document.fonts.ready;
      if (disposed) {
        return;
      }
      const root = document.querySelector<HTMLElement>(".hive-landing");
      if (!root) {
        return;
      }
      gsap.registerPlugin(ScrollTrigger, SplitText);
      const media = gsap.matchMedia();
      const context = gsap.context(() => {
        media.add(
          {
            phone:
              "(prefers-reduced-motion: no-preference) and (max-width: 700px)",
            wide: "(prefers-reduced-motion: no-preference) and (min-width: 701px)",
          },
          (match) => {
            const phone = Boolean(match.conditions?.phone);
            const one = <T extends HTMLElement = HTMLElement>(
              selector: string
            ) => {
              const element = root.querySelector<T>(selector);
              if (!element) {
                throw new Error(
                  `Missing landing animation element: ${selector}`
                );
              }
              return element;
            };
            const all = (selector: string) => [
              ...root.querySelectorAll<HTMLElement>(selector),
            ];
            const smooth = new Lenis({ anchors: { offset: -88 }, lerp: 0.09 });
            smooth.on("scroll", ScrollTrigger.update);
            const tick = (time: number) => smooth.raf(time * 1000);
            gsap.ticker.add(tick);
            const splits: ReturnType<typeof SplitText.create>[] = [];
            for (const element of all("[data-hive-rise]")) {
              const split = SplitText.create(element, {
                mask: "words",
                type: "words",
              });
              splits.push(split);
              gsap.from(split.words, {
                duration: 0.85,
                ease: "power3.out",
                rotation: 3,
                scrollTrigger: {
                  once: true,
                  start: "top 90%",
                  trigger: element,
                },
                stagger: 0.035,
                yPercent: 105,
              });
            }
            for (const element of all("[data-hive-clip]")) {
              gsap.from(element, {
                clipPath: "inset(0 100% 0 0 round 16px)",
                duration: 1,
                ease: "power3.inOut",
                scrollTrigger: {
                  once: true,
                  start: "top 90%",
                  trigger: element,
                },
              });
            }
            for (const [index, band] of all(".network__band").entries()) {
              if (!index) {
                continue;
              }
              const inner = band.querySelector<HTMLElement>(".network__inner");
              if (!inner) {
                continue;
              }
              gsap.fromTo(
                inner,
                {
                  rotate: index % 2 ? 1.6 : -1.6,
                  y: () => -inner.offsetHeight * (phone ? 0.42 : 0.5),
                },
                {
                  ease: "none",
                  rotate: 0,
                  scrollTrigger: {
                    end: "top 50%",
                    invalidateOnRefresh: true,
                    scrub: 0.5,
                    start: "top bottom",
                    trigger: band,
                  },
                  y: 0,
                }
              );
            }
            const workflow = one("#workflow");
            const beds = all(".flow__bed");
            const animateWorkflow = () => {
              const stage = one(".flow__stage");
              const board = one(".flow__board");
              const seams = { a: 25, b: 50, c: 75, x: 50, yl: 50, yr: 50 };
              const write = (
                index: number,
                top: number,
                right: number,
                bottom: number,
                left: number
              ) => {
                const bed = beds[index];
                if (!bed) {
                  return;
                }
                for (const [key, value] of Object.entries({
                  b: bottom,
                  l: left,
                  r: right,
                  t: top,
                })) {
                  bed.style.setProperty(`--flow-${key}`, String(value));
                }
                bed.dataset.expanded = String(
                  phone
                    ? 100 - top - bottom > 40
                    : (100 - left - right) * (100 - top - bottom) > 4000
                );
              };
              const apply = () => {
                if (phone) {
                  write(0, 0, 0, 100 - seams.a, 0);
                  write(1, seams.a, 0, 100 - seams.b, 0);
                  write(2, seams.b, 0, 100 - seams.c, 0);
                  write(3, seams.c, 0, 0, 0);
                } else {
                  write(0, 0, 100 - seams.x, 100 - seams.yl, 0);
                  write(1, 0, 0, 100 - seams.yr, seams.x);
                  write(2, seams.yl, 100 - seams.x, 0, 0);
                  write(3, seams.yr, 0, 0, seams.x);
                }
              };
              const states = phone
                ? [
                    { a: 52, b: 68, c: 84 },
                    { a: 16, b: 68, c: 84 },
                    { a: 16, b: 32, c: 84 },
                    { a: 16, b: 32, c: 48 },
                    { a: 25, b: 50, c: 75 },
                  ]
                : [
                    { x: 68, yl: 74, yr: 50 },
                    { x: 32, yl: 50, yr: 74 },
                    { x: 68, yl: 26, yr: 50 },
                    { x: 32, yl: 50, yr: 26 },
                    { x: 50, yl: 50, yr: 50 },
                  ];
              workflow.dataset.board = "on";
              // CSS offsets contain px units, so Number() is not appropriate.
              const top = () =>
                // oxlint-disable-next-line unicorn/prefer-number-coercion
                Number.parseFloat(getComputedStyle(board).top) || 0;
              const flow = gsap.timeline({
                defaults: { duration: 1, ease: "power2.inOut" },
                onUpdate: apply,
                scrollTrigger: {
                  end: () => `bottom ${top() + board.offsetHeight}px`,
                  invalidateOnRefresh: true,
                  scrub: 0.5,
                  start: () => `top ${top()}px`,
                  trigger: stage,
                },
              });
              flow.to({}, { duration: 0.35 });
              for (const [index, state] of states.entries()) {
                const label = `step-${index}`;
                flow.addLabel(label).to(seams, { ...state }, label);
                if (phone) {
                  for (const [position, bed] of beds.entries()) {
                    const shown =
                      index === states.length - 1 || position === index;
                    flow.to(
                      bed.querySelector(".flow__text"),
                      { autoAlpha: shown ? 1 : 0, duration: 0.5 },
                      `${label}+=${shown ? 0.45 : 0}`
                    );
                  }
                }
                flow.to(
                  {},
                  { duration: index === states.length - 1 ? 0.35 : 0.6 }
                );
              }
            };
            animateWorkflow();
            const animateManifesto = () => {
              const sentence = one(".manifesto__sentence");
              const split = SplitText.create(sentence, {
                type: "words",
                wordsClass: "manifesto__word",
              });
              splits.push(split);
              const words = [
                ...sentence.querySelectorAll<HTMLElement>(
                  ".manifesto__word, .manifesto__pill"
                ),
              ];
              const step = 1 / words.length;
              const read = gsap.timeline({
                defaults: { ease: "none" },
                scrollTrigger: {
                  end: "bottom 64%",
                  scrub: 0.4,
                  start: "top 84%",
                  trigger: sentence,
                },
              });
              for (const [index, word] of words.entries()) {
                if (word.classList.contains("manifesto__pill")) {
                  read.fromTo(
                    word,
                    { clipPath: "inset(0% 50% 0% 50% round 999px)" },
                    {
                      clipPath: "inset(0% 0% 0% 0% round 999px)",
                      duration: step * 4,
                      ease: "power2.out",
                    },
                    index * step
                  );
                  read.fromTo(
                    word.querySelector("img"),
                    { scale: 1.6 },
                    { duration: step * 6, scale: 1 },
                    index * step
                  );
                } else {
                  read.fromTo(
                    word,
                    { opacity: 0.2 },
                    { duration: step * 2.5, opacity: 1 },
                    index * step
                  );
                }
              }
            };
            animateManifesto();
            const showcase = one("#capabilities");
            const items = all(".showcase__item");
            const last = items.at(-1);
            const offset = (index: number) =>
              phone ? 70 + index * 14 : 92 + index * 20;
            showcase.dataset.stack = "on";
            for (const [index, item] of items.entries()) {
              const next = items[index + 1];
              gsap.fromTo(
                item.querySelector(".showcase__img"),
                { yPercent: -4 },
                {
                  ease: "none",
                  scrollTrigger: {
                    end: () => `top ${offset(index)}px`,
                    invalidateOnRefresh: true,
                    scrub: true,
                    start: "top bottom",
                    trigger: item,
                  },
                  yPercent: 4,
                }
              );
              if (!next || !last) {
                continue;
              }
              ScrollTrigger.create({
                end: () => `top ${offset(items.length - 1)}px`,
                endTrigger: last,
                invalidateOnRefresh: true,
                pin: true,
                pinSpacing: false,
                start: () => `top ${offset(index)}px`,
                trigger: item,
              });
              gsap
                .timeline({
                  defaults: { ease: "none" },
                  scrollTrigger: {
                    end: () => `top ${offset(index + 1)}px`,
                    invalidateOnRefresh: true,
                    scrub: true,
                    start: "top bottom",
                    trigger: next,
                  },
                })
                .fromTo(
                  item.querySelector(".showcase__inner"),
                  { scale: 1 },
                  { scale: 0.93 },
                  0
                )
                .fromTo(
                  item.querySelector(".showcase__shade"),
                  { opacity: 0 },
                  { opacity: 0.45 },
                  0
                );
            }
            const creators = one("#creators");
            // Phones stack the chapter panels; the sideways track only runs on wider screens.
            if (!phone) {
              const pin = one(".chapters__pin");
              const track = one(".chapters__track");
              const panels = all(".chapters__panel");
              // Read the panel colors before switching their backgrounds to transparent.
              const grounds = panels.map((panel) => {
                const style = getComputedStyle(panel);
                return {
                  backgroundColor: style.backgroundColor,
                  color: style.color,
                };
              });
              creators.dataset.track = "on";
              const distance = () =>
                Math.max(0, track.scrollWidth - pin.clientWidth);
              gsap.set(pin, grounds[0] ?? {});
              gsap.to(track, {
                ease: "none",
                scrollTrigger: {
                  end: () => `+=${distance()}`,
                  invalidateOnRefresh: true,
                  pin: true,
                  scrub: 0.6,
                  start: "top top",
                  trigger: pin,
                },
                x: () => -distance(),
              });
              const ground = gsap.timeline({
                defaults: { ease: "none" },
                scrollTrigger: {
                  end: () => `+=${distance()}`,
                  invalidateOnRefresh: true,
                  scrub: 0.6,
                  start: "top top",
                  trigger: pin,
                },
              });
              const span = 1 / (panels.length - 1);
              for (const [
                index,
                { backgroundColor, color },
              ] of grounds.entries()) {
                if (!index) {
                  continue;
                }
                const at = (index - 0.62) * span;
                ground
                  .to(pin, { backgroundColor, duration: span * 0.3 }, at)
                  .to(pin, { color, duration: 0.001 }, at + span * 0.3 * 0.62);
              }
              ground.to({}, { duration: 0.001 }, 1);
            }
            const list = one(".assurance__list");
            const cards = all(".assurance__card");

            const pile = () =>
              window.innerWidth <= 860 && cards[0]
                ? centre(cards[0])
                : { x: list.clientWidth / 2, y: list.clientHeight / 2 };
            for (const [index, card] of cards.entries()) {
              gsap.fromTo(
                card,
                {
                  rotate: [-7, 4, 9][index] ?? 0,
                  x: () => pile().x - centre(card).x,
                  y: () => pile().y - centre(card).y + 60,
                },
                {
                  ease: "power1.out",
                  rotate: 0,
                  scrollTrigger: {
                    end: "top 30%",
                    invalidateOnRefresh: true,
                    scrub: 0.5,
                    start: "top 82%",
                    trigger: list,
                  },
                  x: 0,
                  y: 0,
                }
              );
            }
            const join = one("#start");
            const tab = one(".join__tab");
            if (window.innerWidth > 860) {
              gsap
                .timeline({
                  scrollTrigger: {
                    end: "top 20%",
                    invalidateOnRefresh: true,
                    scrub: 0.5,
                    start: "top 90%",
                    trigger: join,
                  },
                })
                .fromTo(
                  one(".join__panel"),
                  { "--hive-shape-trh": "0px", "--hive-shape-trw": "0px" },
                  {
                    "--hive-shape-trh": () => `${tab.offsetHeight}px`,
                    "--hive-shape-trw": () => `${tab.offsetWidth}px`,
                    ease: "power2.inOut",
                  },
                  0
                )
                .fromTo(
                  tab,
                  { x: () => -tab.offsetWidth * 0.35 },
                  { ease: "power2.out", x: 0 },
                  0
                );
            }
            // Sort by document position before measuring pins, as in the reference.
            // A single frame coalesces image loads rather than refreshing mid-layout.
            let refreshFrame = 0;
            const refresh = () => {
              if (disposed || refreshFrame) {
                return;
              }
              refreshFrame = window.requestAnimationFrame(() => {
                refreshFrame = 0;
                if (!disposed) {
                  ScrollTrigger.sort();
                  ScrollTrigger.refresh();
                }
              });
            };
            const images = [...root.querySelectorAll("img")];
            for (const image of images) {
              if (!image.complete) {
                image.addEventListener("load", refresh, { once: true });
              }
            }
            refresh();
            return () => {
              window.cancelAnimationFrame(refreshFrame);
              gsap.ticker.remove(tick);
              smooth.destroy();
              for (const part of splits) {
                part.revert();
              }
              delete workflow.dataset.board;
              for (const bed of beds) {
                delete bed.dataset.expanded;
                for (const key of ["t", "r", "b", "l"]) {
                  bed.style.removeProperty(`--flow-${key}`);
                }
              }
              delete showcase.dataset.stack;
              delete creators.dataset.track;
              for (const image of images) {
                image.removeEventListener("load", refresh);
              }
            };
          },
          root
        );
      }, root);
      teardown = () => {
        media.revert();
        context.revert();
      };
    };
    void setup();
    return () => {
      disposed = true;
      teardown?.();
    };
  }, []);
  return null;
};
export default LandingMotion;
