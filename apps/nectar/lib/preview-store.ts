"use client";

import { useSyncExternalStore } from "react";

import type { Draft, WorkflowDraft } from "../app/dashboard/workspace-pages";
import { capabilities } from "./capabilities";

interface PreviewState {
  budget: number;
  drafts: Draft[];
  storageError: boolean;
  workflow: WorkflowDraft;
  workspaceName: string;
}

const defaults: PreviewState = {
  budget: 100,
  drafts: [],
  storageError: false,
  workflow: {
    name: "My repository check",
    steps: ["repository-analyzer", "sast-scan"],
  },
  workspaceName: "Personal workspace",
};
const storageKey = "hive-preview-workspace-v1";
const changeEvent = "hive-preview-changed";
let snapshot = defaults;
let lastRaw: string | null | undefined;

const isDraft = (value: unknown): value is Draft => {
  if (!value || typeof value !== "object") {
    return false;
  }
  return (
    "id" in value &&
    typeof value.id === "string" &&
    "name" in value &&
    typeof value.name === "string" &&
    "description" in value &&
    typeof value.description === "string" &&
    "type" in value &&
    typeof value.type === "string" &&
    "price" in value &&
    typeof value.price === "number" &&
    Number.isFinite(value.price) &&
    value.price >= 0
  );
};

const parseState = (raw: string | null): PreviewState => {
  if (!raw) {
    return defaults;
  }
  const value: unknown = JSON.parse(raw);
  if (!value || typeof value !== "object") {
    return defaults;
  }
  const state = { ...defaults };
  if (
    "budget" in value &&
    typeof value.budget === "number" &&
    Number.isFinite(value.budget) &&
    value.budget >= 0
  ) {
    state.budget = value.budget;
  }
  if (
    "workspaceName" in value &&
    typeof value.workspaceName === "string" &&
    value.workspaceName.trim()
  ) {
    state.workspaceName = value.workspaceName;
  }
  if ("drafts" in value && Array.isArray(value.drafts)) {
    state.drafts = value.drafts.filter(isDraft);
  }
  if (
    "workflow" in value &&
    value.workflow &&
    typeof value.workflow === "object"
  ) {
    const flow = value.workflow;
    if (
      "name" in flow &&
      typeof flow.name === "string" &&
      "steps" in flow &&
      Array.isArray(flow.steps)
    ) {
      state.workflow = {
        name: flow.name,
        steps: flow.steps.filter(
          (step): step is string =>
            typeof step === "string" &&
            capabilities.some((item) => item.id === step)
        ),
      };
    }
  }
  return state;
};

const getSnapshot = () => {
  try {
    const raw = localStorage.getItem(storageKey);
    if (raw !== lastRaw) {
      lastRaw = raw;
      snapshot = parseState(raw);
    }
  } catch {
    if (!snapshot.storageError) {
      snapshot = { ...snapshot, storageError: true };
    }
  }
  return snapshot;
};
const getServerSnapshot = () => defaults;
const subscribe = (onStoreChange: () => void) => {
  const onStorage = (event: StorageEvent) => {
    if (event.key === storageKey || event.key === null) {
      onStoreChange();
    }
  };
  window.addEventListener("storage", onStorage);
  window.addEventListener(changeEvent, onStoreChange);
  return () => {
    window.removeEventListener("storage", onStorage);
    window.removeEventListener(changeEvent, onStoreChange);
  };
};

export const updatePreview = (
  update:
    | Partial<PreviewState>
    | ((current: PreviewState) => Partial<PreviewState>)
) => {
  const current = getSnapshot();
  snapshot = {
    ...current,
    ...(typeof update === "function" ? update(current) : update),
  };
  try {
    const raw = JSON.stringify(snapshot);
    localStorage.setItem(storageKey, raw);
    lastRaw = raw;
  } catch {
    snapshot = { ...snapshot, storageError: true };
  }
  window.dispatchEvent(new Event(changeEvent));
};

export const usePreview = () =>
  useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
