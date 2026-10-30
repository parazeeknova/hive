"use client";

import { useCallback, useSyncExternalStore } from "react";

import type { RegistryCapability } from "./hiveway";

export interface Profile {
  avatar: string;
  name: string;
  username: string;
}
export interface CapabilityPreference {
  active: boolean;
  paid: boolean;
  paidPrice: string;
}
export interface AccountPreview {
  balance: string;
  capabilities: Record<string, CapabilityPreference>;
  profile: Profile;
  storageError: boolean;
  welcomeDismissed: boolean;
}

const defaults: AccountPreview = {
  balance: "50000000",
  capabilities: {},
  profile: { avatar: "", name: "Hive member", username: "hive-member" },
  storageError: false,
  welcomeDismissed: false,
};
const eventName = "hive-account-preview-changed";
const records = new Map<
  string,
  { raw: string | null; state: AccountPreview }
>();
const keyFor = (scope: string) =>
  `hive-account-preview-v1:${scope.toLowerCase()}`;
const isAmount = (value: unknown): value is string =>
  typeof value === "string" && /^\d{1,15}$/u.test(value);

const readProfile = (value: object): Profile => {
  if (
    !("name" in value) ||
    typeof value.name !== "string" ||
    !value.name.trim() ||
    value.name.length > 60 ||
    !("username" in value) ||
    typeof value.username !== "string" ||
    !/^[a-z\d_-]{3,30}$/u.test(value.username)
  ) {
    return defaults.profile;
  }
  const avatar =
    "avatar" in value &&
    typeof value.avatar === "string" &&
    /^data:image\/(?:png|jpeg|webp);base64,[a-z\d+/=]+$/iu.test(value.avatar) &&
    value.avatar.length <= 1_400_000
      ? value.avatar
      : "";
  return { avatar, name: value.name.trim(), username: value.username };
};

export const parseAccountPreview = (raw: string | null): AccountPreview => {
  if (!raw) {
    return defaults;
  }
  try {
    const value: unknown = JSON.parse(raw);
    if (!value || typeof value !== "object") {
      return defaults;
    }
    const state = { ...defaults };
    if (
      "welcomeDismissed" in value &&
      typeof value.welcomeDismissed === "boolean"
    ) {
      state.welcomeDismissed = value.welcomeDismissed;
    }
    if ("balance" in value && isAmount(value.balance)) {
      state.balance = value.balance;
    }
    if (
      "profile" in value &&
      value.profile &&
      typeof value.profile === "object"
    ) {
      state.profile = readProfile(value.profile);
    }
    if (
      "capabilities" in value &&
      value.capabilities &&
      typeof value.capabilities === "object"
    ) {
      state.capabilities = Object.fromEntries(
        Object.entries(value.capabilities).filter(
          ([, pref]) =>
            pref &&
            typeof pref === "object" &&
            typeof pref.active === "boolean" &&
            typeof pref.paid === "boolean" &&
            isAmount(pref.paidPrice) &&
            BigInt(pref.paidPrice) > 0n &&
            BigInt(pref.paidPrice) <= 1_000_000_000n
        )
      );
    }
    return state;
  } catch {
    return defaults;
  }
};

const getSnapshot = (scope: string) => {
  const key = keyFor(scope);
  const previous = records.get(key);
  try {
    const raw = localStorage.getItem(key);
    if (!previous || raw !== previous.raw) {
      const state = parseAccountPreview(raw);
      records.set(key, { raw, state });
      return state;
    }
    return previous.state;
  } catch {
    if (previous?.state.storageError) {
      return previous.state;
    }
    const state = { ...(previous?.state ?? defaults), storageError: true };
    records.set(key, { raw: previous?.raw ?? null, state });
    return state;
  }
};

export const updateAccountPreview = (
  scope: string,
  update: (current: AccountPreview) => AccountPreview
) => {
  const current = getSnapshot(scope);
  let state = update(current);
  let raw: string | null = null;
  try {
    raw = JSON.stringify(state);
    localStorage.setItem(keyFor(scope), raw);
  } catch {
    raw = records.get(keyFor(scope))?.raw ?? null;
    state = { ...state, storageError: true };
  }
  records.set(keyFor(scope), { raw, state });
  window.dispatchEvent(new Event(eventName));
};

export const useAccountPreview = (scope: string) => {
  const subscribe = useCallback(
    (notify: () => void) => {
      const onStorage = (event: StorageEvent) => {
        if (event.key === keyFor(scope) || event.key === null) {
          notify();
        }
      };
      window.addEventListener("storage", onStorage);
      window.addEventListener(eventName, notify);
      return () => {
        window.removeEventListener("storage", onStorage);
        window.removeEventListener(eventName, notify);
      };
    },
    [scope]
  );
  const snapshot = useCallback(() => getSnapshot(scope), [scope]);
  const updateAccount = useCallback(
    (update: (current: AccountPreview) => AccountPreview) =>
      updateAccountPreview(scope, update),
    [scope]
  );
  return {
    account: useSyncExternalStore(subscribe, snapshot, () => defaults),
    updateAccount,
  };
};

export const preferenceFor = (
  tool: RegistryCapability,
  preferences: AccountPreview["capabilities"]
): CapabilityPreference =>
  (Object.hasOwn(preferences, tool.id) ? preferences[tool.id] : undefined) ?? {
    active: tool.active,
    paid: BigInt(tool.price) > 0n,
    paidPrice: BigInt(tool.price) > 0n ? tool.price : "10000",
  };

export const resolveAccountCapability = (
  tool: RegistryCapability,
  preferences: AccountPreview["capabilities"]
) => {
  const pref = preferenceFor(tool, preferences);
  return {
    ...tool,
    active: pref.active,
    price: pref.paid ? pref.paidPrice : "0",
  };
};

export const balanceHealth = (amount: string, target = "50000000") => {
  const available = BigInt(amount);
  const cap = BigInt(target);
  const percent = cap > 0n ? Number((available * 100n) / cap) : 0;
  if (available <= 0n) {
    return { label: "Empty", percent: 0, tone: "low" };
  }
  let label = "Ready to run";
  let tone = "healthy";
  if (available * 100n <= cap * 20n) {
    label = "Running low";
    tone = "low";
  } else if (available * 100n <= cap * 50n) {
    label = "Getting low";
    tone = "medium";
  }
  return {
    label,
    percent: Math.min(100, Math.max(1, percent)),
    tone,
  };
};

export const addDemoFunds = (scope: string, amount: string) => {
  if (
    scope !== "preview" ||
    !isAmount(amount) ||
    BigInt(amount) < 1n ||
    BigInt(amount) > 1_000_000_000n
  ) {
    throw new Error(
      "Demo top-ups are only available in preview mode, up to $1,000."
    );
  }
  updateAccountPreview(scope, (current) => ({
    ...current,
    balance: (BigInt(current.balance) + BigInt(amount)).toString(),
  }));
};

export const parseTopUp = (value: string) => {
  if (!/^\d+(?:\.\d{1,2})?$/u.test(value)) {
    throw new Error("Enter a USD amount with up to two decimal places.");
  }
  const [whole = "0", cents = ""] = value.split(".");
  const amount = BigInt(whole) * 1_000_000n + BigInt(cents.padEnd(6, "0"));
  if (amount < 1_000_000n || amount > 1_000_000_000n) {
    throw new Error("Choose an amount between $1 and $1,000.");
  }
  return amount.toString();
};
