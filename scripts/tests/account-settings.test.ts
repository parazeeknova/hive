/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";

import {
  addDemoFunds,
  balanceHealth,
  parseAccountPreview,
  parseTopUp,
  preferenceFor,
  resolveAccountCapability,
  updateAccountPreview,
} from "../../apps/nectar/lib/account-store";
import { exampleCapabilities } from "../../apps/nectar/lib/hiveway";

afterEach(() => {
  mock.restore();
});

describe("account settings boundaries", () => {
  test("starts with $50 and dismisses the prompt without removing credit", () => {
    const account = parseAccountPreview(null);
    expect(account.balance).toBe("50000000");
    expect(account.welcomeDismissed).toBe(false);
    const saved = parseAccountPreview(
      JSON.stringify({ ...account, welcomeDismissed: true })
    );
    expect(saved.welcomeDismissed).toBe(true);
    expect(saved.balance).toBe("50000000");
  });

  test("free/paid retains the paid price and disabled items stay manageable", () => {
    const [tool] = exampleCapabilities;
    if (!tool) {
      throw new Error("Missing example capability");
    }
    const pref = {
      ...preferenceFor(tool, {}),
      active: false,
      paid: false,
      paidPrice: "290000",
    };
    const preferences = { [tool.id]: pref };
    const free = resolveAccountCapability(tool, preferences);
    expect(free.price).toBe("0");
    expect(free.active).toBe(false);
    expect(free.id).toBe(tool.id);
    const paid = resolveAccountCapability(tool, {
      [tool.id]: { ...pref, paid: true },
    });
    expect(paid.price).toBe("290000");
    expect(tool.active).toBe(true);
    expect(tool.price).toBe("10000");
  });

  test("balance health has accessible states and clamps the visual bar", () => {
    expect(balanceHealth("0")).toEqual({
      label: "Empty",
      percent: 0,
      tone: "low",
    });
    expect(balanceHealth("10000000").tone).toBe("low");
    expect(balanceHealth("10000001").tone).toBe("medium");
    expect(balanceHealth("10500000").tone).toBe("medium");
    expect(balanceHealth("25000000").tone).toBe("medium");
    expect(balanceHealth("50000000")).toEqual({
      label: "Ready to run",
      percent: 100,
      tone: "healthy",
    });
    expect(balanceHealth("100000000").percent).toBe(100);
  });

  test("rejects invalid stored pricing and unsafe avatar sources", () => {
    const state = parseAccountPreview(
      JSON.stringify({
        balance: "-100",
        capabilities: {
          bad: { active: true, paid: true, paidPrice: "-1" },
          good: { active: false, paid: false, paidPrice: "1" },
        },
        profile: {
          avatar: "https://untrusted.example/avatar.svg",
          name: "Ari",
          username: "ari_dev",
        },
      })
    );
    expect(state.balance).toBe("50000000");
    expect(state.profile.avatar).toBe("");
    expect(Object.keys(state.capabilities)).toEqual(["good"]);
    expect(parseAccountPreview("not json").balance).toBe("50000000");
  });

  test("top-up amounts preserve cents and reject unsupported values", () => {
    expect(parseTopUp("25")).toBe("25000000");
    expect(parseTopUp("10.29")).toBe("10290000");
    for (const value of [
      "0",
      "0.99",
      "-5",
      "1e2",
      "25.001",
      "1000.01",
      "NaN",
    ]) {
      expect(() => parseTopUp(value)).toThrow();
    }
    expect(() => addDemoFunds("0xwallet", "25000000")).toThrow();
  });

  test("keeps wallets isolated and demo credits local to the preview", () => {
    const storage = new Map<string, string>();
    const oldStorage = Object.getOwnPropertyDescriptor(
      globalThis,
      "localStorage"
    );
    const oldWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: {
        getItem: (key: string) => storage.get(key) ?? null,
        setItem: (key: string, value: string) => {
          storage.set(key, value);
        },
      },
    });
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: new EventTarget(),
    });
    try {
      updateAccountPreview("wallet-a", (account) => ({
        ...account,
        welcomeDismissed: true,
      }));
      updateAccountPreview("wallet-b", (account) => ({
        ...account,
        profile: { ...account.profile, name: "Bee" },
      }));
      addDemoFunds("preview", "25000000");
      const read = (scope: string) =>
        parseAccountPreview(
          storage.get(`hive-account-preview-v1:${scope}`) ?? null
        );
      expect(read("wallet-a").welcomeDismissed).toBe(true);
      expect(read("wallet-b").welcomeDismissed).toBe(false);
      expect(read("wallet-a").profile.name).toBe("Hive member");
      expect(read("preview").balance).toBe("75000000");
      expect(read("wallet-a").balance).toBe("50000000");
    } finally {
      if (oldStorage) {
        Object.defineProperty(globalThis, "localStorage", oldStorage);
      } else {
        Reflect.deleteProperty(globalThis, "localStorage");
      }
      if (oldWindow) {
        Object.defineProperty(globalThis, "window", oldWindow);
      } else {
        Reflect.deleteProperty(globalThis, "window");
      }
    }
  });
});
