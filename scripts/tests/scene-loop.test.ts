/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";

import { observeLoopInView } from "../../apps/nectar/lib/hooks/use-loop-in-view";

interface ElementStub extends EventTarget {
  dataset: Record<string, string>;
  card: ElementStub | null;
  closest: () => ElementStub | null;
}
const createElement = (): ElementStub =>
  Object.assign(new EventTarget(), {
    card: null as ElementStub | null,
    closest() {
      return this.card;
    },
    dataset: { loop: "idle" } as Record<string, string>,
  });

const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
const originalDocument = Object.getOwnPropertyDescriptor(
  globalThis,
  "document"
);

afterEach(() => {
  if (originalWindow) {
    Object.defineProperty(globalThis, "window", originalWindow);
  } else {
    Reflect.deleteProperty(globalThis, "window");
  }
  if (originalDocument) {
    Object.defineProperty(globalThis, "document", originalDocument);
  } else {
    Reflect.deleteProperty(globalThis, "document");
  }
});

const setup = ({ card = false, reduced = false } = {}) => {
  const root = createElement();
  if (card) {
    root.card = createElement();
  }
  const motion = Object.assign(new EventTarget(), { matches: reduced });
  const documentStub = Object.assign(new EventTarget(), { hidden: false });
  const frames = new Map<number, FrameRequestCallback>();
  let frameId = 0;
  const observers: ObserverStub[] = [];
  class ObserverStub {
    observed: unknown;
    disconnected = false;
    readonly onIntersect: IntersectionObserverCallback;
    readonly thresholds: number | number[] | undefined;
    constructor(
      onIntersect: IntersectionObserverCallback,
      options?: IntersectionObserverInit
    ) {
      this.onIntersect = onIntersect;
      this.thresholds = options?.threshold;
      observers.push(this);
    }
    observe(target: unknown) {
      this.observed = target;
    }
    disconnect() {
      this.disconnected = true;
    }
  }
  const windowStub = {
    IntersectionObserver: ObserverStub,
    cancelAnimationFrame: (id: number) => {
      frames.delete(id);
    },
    matchMedia: () => motion,
    requestAnimationFrame: (next: FrameRequestCallback) => {
      frameId += 1;
      frames.set(frameId, next);
      return frameId;
    },
  };
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: windowStub,
  });
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: documentStub,
  });
  // The browser exposes IntersectionObserver globally as well as on window.
  const originalObserver = Object.getOwnPropertyDescriptor(
    globalThis,
    "IntersectionObserver"
  );
  Object.defineProperty(globalThis, "IntersectionObserver", {
    configurable: true,
    value: ObserverStub,
  });
  const cleanup = observeLoopInView(root as unknown as HTMLElement);
  const intersect = (ratio: number) => {
    observers[0]?.onIntersect(
      [
        {
          intersectionRatio: ratio,
          isIntersecting: ratio > 0,
        } as IntersectionObserverEntry,
      ],
      {} as IntersectionObserver
    );
  };
  const flush = () => {
    for (let pass = 0; pass < 2; pass += 1) {
      const batch = [...frames.entries()];
      frames.clear();
      for (const [, onFrame] of batch) {
        onFrame(0);
      }
    }
  };
  const restore = () => {
    cleanup?.();
    if (originalObserver) {
      Object.defineProperty(
        globalThis,
        "IntersectionObserver",
        originalObserver
      );
    } else {
      Reflect.deleteProperty(globalThis, "IntersectionObserver");
    }
  };
  return {
    get disconnected() {
      return observers[0]?.disconnected;
    },
    documentStub,
    flush,
    frames,
    intersect,
    motion,
    get observed() {
      return observers[0]?.observed;
    },
    root,
    stop: restore,
    get thresholds() {
      return observers[0]?.thresholds;
    },
  };
};

describe("scene visibility lifecycle", () => {
  test("standalone scene starts only at 40 percent visibility", () => {
    const scene = setup();
    try {
      expect(scene.observed).toBe(scene.root);
      expect(scene.thresholds).toEqual([0, 0.15, 0.4]);
      scene.intersect(0.39);
      scene.flush();
      expect(scene.root.dataset.loop).toBe("idle");
      scene.intersect(0.4);
      scene.flush();
      expect(scene.root.dataset.loop).toBe("running");
    } finally {
      scene.stop();
    }
  });
  test("card entrance must finish before its timeline starts", () => {
    const scene = setup({ card: true });
    try {
      scene.intersect(0.5);
      scene.flush();
      expect(scene.root.card?.dataset.cardReveal).toBe("ready");
      expect(scene.root.dataset.loop).toBe("idle");
      const event = Object.assign(new Event("animationend"), {
        animationName: "network-card-enter",
      });
      scene.root.card?.dispatchEvent(event);
      scene.flush();
      expect(scene.root.dataset.loop).toBe("running");
    } finally {
      scene.stop();
    }
  });
  test("offscreen pauses, then re-entry resets before restarting", () => {
    const scene = setup();
    try {
      scene.intersect(0.5);
      scene.flush();
      scene.intersect(0);
      expect(scene.root.dataset.loop).toBe("paused");
      scene.intersect(0.2);
      scene.flush();
      expect(scene.root.dataset.loop).toBe("paused");
      scene.intersect(0.4);
      expect(scene.root.dataset.loop).toBe("reset");
      scene.flush();
      expect(scene.root.dataset.loop).toBe("running");
    } finally {
      scene.stop();
    }
  });
  test("reduced motion renders statically and preference changes restart safely", () => {
    const scene = setup({ reduced: true });
    try {
      scene.intersect(0.5);
      scene.flush();
      expect(scene.root.dataset.loop).toBe("static");
      expect(scene.frames.size).toBe(0);
      scene.motion.matches = false;
      scene.motion.dispatchEvent(new Event("change"));
      scene.flush();
      expect(scene.root.dataset.loop).toBe("running");
      scene.motion.matches = true;
      scene.motion.dispatchEvent(new Event("change"));
      expect(scene.root.dataset.loop).toBe("static");
    } finally {
      scene.stop();
    }
  });
  test("hidden documents pause and teardown cancels pending restart frames", () => {
    const scene = setup();
    scene.intersect(0.5);
    scene.flush();
    scene.documentStub.hidden = true;
    scene.documentStub.dispatchEvent(new Event("visibilitychange"));
    expect(scene.root.dataset.loop).toBe("paused");
    scene.documentStub.hidden = false;
    scene.documentStub.dispatchEvent(new Event("visibilitychange"));
    expect(scene.frames.size).toBe(1);
    scene.stop();
    expect(scene.disconnected).toBe(true);
    expect(scene.frames.size).toBe(0);
  });
});
