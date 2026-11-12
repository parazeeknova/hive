"use client";
import { useEffect, useRef } from "react";

/**
 * Muted, looping hero footage. The poster shows first; playback is skipped for
 * reduced-motion and data-saver visitors, and pauses while off-screen.
 */
const HeroVideo = ({ className }: { className?: string }) => {
  const video = useRef<HTMLVideoElement>(null);
  useEffect(() => {
    const element = video.current;
    if (!element) {
      return;
    }
    const still = window.matchMedia("(prefers-reduced-motion: reduce)");
    const saveData =
      "connection" in navigator &&
      Boolean((navigator.connection as { saveData?: boolean }).saveData);
    if (saveData) {
      return;
    }
    let visible = true;
    const sync = () => {
      if (still.matches || !visible) {
        element.pause();
      } else {
        element.play().catch(() => null);
      }
    };
    const observer = new IntersectionObserver(([entry]) => {
      visible = Boolean(entry?.isIntersecting);
      sync();
    });
    observer.observe(element);
    still.addEventListener("change", sync);
    sync();
    return () => {
      observer.disconnect();
      still.removeEventListener("change", sync);
    };
  }, []);
  return (
    <video
      ref={video}
      className={className}
      poster="/landing/bee-hero-poster.webp"
      muted
      loop
      playsInline
      preload="metadata"
      aria-hidden="true"
    >
      <source src="/landing/bee-hero.mp4" type="video/mp4" />
    </video>
  );
};
export default HeroVideo;
