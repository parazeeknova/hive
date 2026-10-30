"use client";

import { useEffect, useRef } from "react";

/** Coordinate CSS timelines without a per-frame JS render loop. */
export const observeLoopInView = (root: HTMLElement) => {
  const card = root.closest<HTMLElement>("[data-scene-card]");
  const target = card ?? root;
  const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
  let inView = false;
  let entered = !card || card.dataset.entered === "true";
  let resetFrame = 0;
  let startFrame = 0;

  const cancelFrames = () => {
    window.cancelAnimationFrame(resetFrame);
    window.cancelAnimationFrame(startFrame);
  };
  const pause = () => {
    cancelFrames();
    if (root.dataset.loop === "running" || root.dataset.loop === "reset") {
      root.dataset.loop = "paused";
    }
  };
  const start = () => {
    if (motion.matches) {
      cancelFrames();
      root.dataset.loop = "static";
      return;
    }
    if (
      !inView ||
      !entered ||
      document.hidden ||
      root.dataset.loop === "running"
    ) {
      return;
    }
    cancelFrames();
    root.dataset.loop = "reset";
    // Two frames remove the old animation without a synchronous layout read.
    resetFrame = window.requestAnimationFrame(() => {
      startFrame = window.requestAnimationFrame(() => {
        root.dataset.loop = "running";
      });
    });
  };
  const onEntrance = (event: AnimationEvent) => {
    if (event.target !== card || event.animationName !== "network-card-enter") {
      return;
    }
    entered = true;
    if (card) {
      card.dataset.entered = "true";
    }
    start();
  };
  const onMotion = () => {
    if (motion.matches) {
      entered = true;
      if (card) {
        card.dataset.cardReveal = "ready";
        card.dataset.entered = "true";
      }
    }
    start();
  };
  const onVisibility = () => {
    if (document.hidden) {
      pause();
    } else {
      start();
    }
  };

  if (!("IntersectionObserver" in window)) {
    root.dataset.loop = "static";
    if (card) {
      card.dataset.cardReveal = "ready";
      card.dataset.entered = "true";
    }
    return;
  }
  const observer = new IntersectionObserver(
    (entries) => {
      const entry = entries.at(-1);
      if (!entry) {
        return;
      }
      if (!entry.isIntersecting || entry.intersectionRatio === 0) {
        inView = false;
        pause();
        return;
      }
      if (card && entry.intersectionRatio >= 0.15) {
        card.dataset.cardReveal = "ready";
      }
      if (entry.intersectionRatio >= 0.4) {
        inView = true;
        start();
      }
    },
    { threshold: [0, 0.15, 0.4] }
  );
  card?.addEventListener("animationend", onEntrance);
  motion.addEventListener("change", onMotion);
  document.addEventListener("visibilitychange", onVisibility);
  observer.observe(target);
  onMotion();

  return () => {
    cancelFrames();
    observer.disconnect();
    card?.removeEventListener("animationend", onEntrance);
    motion.removeEventListener("change", onMotion);
    document.removeEventListener("visibilitychange", onVisibility);
  };
};

/** Return a scene ref; observe its containing card, or the scene when standalone. */
export const useLoopInView = <T extends HTMLElement = HTMLElement>() => {
  const ref = useRef<T>(null);
  useEffect(() => {
    const root = ref.current;
    if (root) {
      return observeLoopInView(root);
    }
  }, []);
  return ref;
};
