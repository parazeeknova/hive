"use client";

import Image from "next/image";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";

import { Brand } from "../../components/brand";
import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";
import { toolIconBase, toolTint } from "../../components/tool-icon";
import { capabilities, money } from "../../lib/capabilities";
import type { Capability } from "../../lib/capabilities";
import { updatePreview, usePreview } from "../../lib/preview-store";
import {
  Billing,
  Composer,
  Executions,
  MyTools,
  Publisher,
  Settings,
} from "./workspace-pages";
import type { Execution } from "./workspace-pages";

type View =
  | "discover"
  | "compose"
  | "tools"
  | "executions"
  | "billing"
  | "settings"
  | "publish"
  | "use";
type Modal = "sample" | "invoke" | null;
const navigation: { id: View; label: string; icon: IconName }[] = [
  { icon: "search", id: "discover", label: "Discover" },
  { icon: "compose", id: "compose", label: "Compose" },
  { icon: "box", id: "tools", label: "My tools" },
  { icon: "activity", id: "executions", label: "Executions" },
  { icon: "wallet", id: "billing", label: "Billing" },
  { icon: "settings", id: "settings", label: "Settings" },
];
const tabs: { id: View; label: string; icon: IconName }[] = [
  { icon: "search", id: "discover", label: "Discover" },
  { icon: "play", id: "use", label: "Use" },
  { icon: "compose", id: "compose", label: "Compose" },
  { icon: "plus", id: "publish", label: "Publish" },
];
const filters = ["All", "APIs", "MCP Tools", "Workflows", "Agents"] as const;
const filterTypes: Record<string, string> = {
  APIs: "API",
  Agents: "Agent",
  "MCP Tools": "MCP Tool",
  Workflows: "Workflow",
};
const [firstCapability] = capabilities;
if (!firstCapability) {
  throw new Error("The preview catalog needs a capability.");
}

const isView = (value: string | null): value is View =>
  [...navigation, ...tabs].some((item) => item.id === value);

const tagPillBase =
  "rounded-sm border border-stone-200 bg-stone-100 px-1.5 py-1 text-xs leading-none text-stone-500 max-[760px]:px-1 max-[760px]:text-xs";
const tagPillSelectedBase = "border-amber-200 bg-amber-50 text-amber-800";

export const Verification = ({ verified }: { verified: boolean }) => (
  <span
    className={`inline-flex items-center gap-1 text-xs whitespace-nowrap ${
      verified ? "text-lime-700" : "text-stone-400"
    }`}
  >
    <Icon name={verified ? "shield" : "globe"} size={11} />
    {verified ? "Verified" : "Community"}
  </span>
);

const ToolDetail = ({
  tool,
  onSample,
  onInvoke,
  onSelect,
}: {
  tool: Capability;
  onSample: () => void;
  onInvoke: () => void;
  onSelect: (tool: Capability) => void;
}) => {
  const schema = {
    properties: {
      [tool.input]: { format: "uri", type: "string" },
      revision: { type: "string" },
    },
    required: [tool.input, "revision"],
    type: "object",
  };
  return (
    <aside
      className="shadow-soft sticky top-5 self-start overflow-hidden rounded-xl border border-stone-200 bg-white max-[1000px]:static"
      aria-label={`${tool.name} details`}
    >
      <div className="relative flex h-24 items-start justify-between border-b border-stone-200 bg-[radial-gradient(ellipse_at_80%_0,#f8e8b6,transparent_65%),linear-gradient(140deg,#f0f3e4,#fbf7e8)] px-4 py-4">
        <span className="text-xs tracking-wide text-stone-400 uppercase">
          A closer look
        </span>
        <Verification verified={tool.verified} />
        <span
          className={`${toolIconBase} ${toolTint(tool.color)} shadow-soft absolute bottom-[-21px] size-12 rounded-xl border-4 border-white bg-white`}
        >
          <Icon name={tool.icon} size={25} />
        </span>
      </div>
      <div className="px-5 pt-9 pb-5 max-[1200px]:px-4 max-[1000px]:px-6 max-[1000px]:pt-9 max-[1000px]:pb-6">
        <div className="flex items-center justify-between gap-2">
          <h2 className="text-xl font-semibold tracking-tight">{tool.name}</h2>
          <span className="rounded-sm border border-stone-200 px-1.5 py-1 text-xs leading-none text-stone-400">
            v1.0
          </span>
        </div>
        <p className="mt-3 text-xs leading-loose text-stone-500">
          {tool.description}
        </p>
        <div className="mt-3 flex flex-wrap gap-1.5">
          <span className={tagPillBase}>{tool.type}</span>
          {tool.tags.map((tag) => (
            <span className={tagPillBase} key={tag}>
              {tag}
            </span>
          ))}
        </div>
        <div className="mt-6 flex border-y border-stone-200 py-4">
          <div className="flex flex-1 flex-col gap-1.5">
            <strong className="text-xl font-semibold text-stone-700">
              {money(tool.price)}
              <small className="text-xs font-normal"> / call</small>
            </strong>
            <span className="text-xs text-stone-500">
              Illustrative price, before invocation
            </span>
          </div>
          <div className="flex flex-1 flex-col items-end gap-1.5 border-l border-stone-200">
            <strong className="flex items-center gap-1 pt-1 text-xs font-semibold text-stone-700">
              <Icon name="clock" size={13} /> ~{tool.seconds}s
            </strong>
            <span className="text-xs text-stone-500">Estimated runtime</span>
          </div>
        </div>
        <div className="flex items-center justify-between px-0 pt-3.5 pb-1 text-xs text-stone-500">
          <Verification verified={tool.verified} />
          <span>
            {tool.verified ? "Example contract checks" : "Community listing"}
          </span>
        </div>
        <div className="mt-1.5">
          <details className="border-b border-stone-200">
            <summary className="flex list-none items-center gap-2 py-3.5 text-xs font-medium text-lime-800 [&::-webkit-details-marker]:hidden">
              Overview <Icon name="chevron" size={12} />
            </summary>
            <p className="pb-3 text-xs leading-loose text-stone-500">
              Send a public repository URL and a revision. Receive a structured
              result for that input, ready to use in your next step. This
              listing is an example of the planned registry.
            </p>
          </details>
          <details className="border-b border-stone-200">
            <summary className="flex list-none items-center gap-2 py-3.5 text-xs font-medium text-lime-800 [&::-webkit-details-marker]:hidden">
              Input / output schema <Icon name="chevron" size={12} />
            </summary>
            <p className="pb-3 text-xs leading-loose text-stone-500">
              Input contract
            </p>
            <pre className="mb-3 overflow-x-auto rounded-md bg-stone-100 p-3 text-xs break-words whitespace-pre-wrap text-lime-800">
              {JSON.stringify(schema, null, 2)}
            </pre>
            <p className="pb-3 text-xs leading-loose text-stone-500">
              Output shape (sample)
            </p>
            <pre className="mb-3 overflow-x-auto rounded-md bg-stone-100 p-3 text-xs break-words whitespace-pre-wrap text-lime-800">
              {JSON.stringify(tool.output, null, 2)}
            </pre>
          </details>
          <details className="border-b border-stone-200">
            <summary className="flex list-none items-center gap-2 py-3.5 text-xs font-medium text-lime-800 [&::-webkit-details-marker]:hidden">
              Sample response <Icon name="chevron" size={12} />
            </summary>
            <pre className="mb-3 overflow-x-auto rounded-md bg-stone-100 p-3 text-xs break-words whitespace-pre-wrap text-lime-800">
              {JSON.stringify(tool.output, null, 2)}
            </pre>
          </details>
          <details
            className="border-b border-stone-200"
            open={tool.dependencies.length > 0}
          >
            <summary className="flex list-none items-center gap-2 py-3.5 text-xs font-medium text-lime-800 [&::-webkit-details-marker]:hidden">
              Dependencies <span>({tool.dependencies.length})</span>
              <Icon name="chevron" size={12} />
            </summary>
            {tool.dependencies.length ? (
              tool.dependencies.map((name) => {
                const dependency = capabilities.find(
                  (item) => item.name === name
                );
                return dependency ? (
                  <button
                    className="mb-2 flex w-full items-center justify-between rounded-md border border-stone-200 bg-stone-100 p-2 text-xs text-lime-800"
                    type="button"
                    key={name}
                    onClick={() => onSelect(dependency)}
                  >
                    <span>{name}</span>
                    <span>
                      {money(dependency.price)}{" "}
                      <span aria-hidden="true">↗</span>
                    </span>
                  </button>
                ) : null;
              })
            ) : (
              <p className="pb-3 text-xs leading-loose text-stone-500">
                No downstream capabilities in this example.
              </p>
            )}
          </details>
          <details className="border-b border-stone-200">
            <summary className="flex list-none items-center gap-2 py-3.5 text-xs font-medium text-lime-800 [&::-webkit-details-marker]:hidden">
              Pricing <Icon name="chevron" size={12} />
            </summary>
            <p className="pb-3 text-xs leading-loose text-stone-500">
              Example all-in quote: {money(tool.price)} per successful
              invocation
              {tool.dependencies.length ? ", including downstream calls" : ""}.
              Dependency listings show their standalone prices. Live quotes,
              failed-call terms, and payment authorization will be shown before
              purchase. Sample responses are free and incur no charges.
            </p>
          </details>
          <details className="border-b border-stone-200">
            <summary className="flex list-none items-center gap-2 py-3.5 text-xs font-medium text-lime-800 [&::-webkit-details-marker]:hidden">
              Limits &amp; test evidence <Icon name="chevron" size={12} />
            </summary>
            <p className="pb-3 text-xs leading-loose text-stone-500">
              Public repositories only. A revision is required for reproducible
              inputs. Runtime and repository-size limits will accompany a live
              listing.{" "}
              {tool.verified
                ? "This example’s verified badge represents contract checks; inspect the real checks and timestamps before relying on a published tool."
                : "This community example has no verification evidence."}
            </p>
          </details>
          <details className="border-b border-stone-200">
            <summary className="flex list-none items-center gap-2 py-3.5 text-xs font-medium text-lime-800 [&::-webkit-details-marker]:hidden">
              Provider <Icon name="chevron" size={12} />
            </summary>
            <p className="pb-3 text-xs leading-loose text-stone-500">
              {tool.provider} · Example provider. Version 1.0. Live provider
              details and contact information will appear with published
              listings.
            </p>
          </details>
        </div>
        <div className="mt-5 flex flex-col gap-2 max-[1000px]:flex-row max-[760px]:flex-col">
          <button
            type="button"
            className="inline-flex min-h-11 w-full items-center justify-center gap-3 rounded-lg border border-amber-300 bg-[linear-gradient(135deg,#ffe3a0,#f5ca69)] p-3 text-xs font-bold text-amber-900 transition-[transform,box-shadow,background] duration-200 hover:-translate-y-0.5 active:translate-y-0"
            onClick={onSample}
          >
            <Icon name="play" size={14} /> Test with sample input{" "}
            <Icon name="arrow" size={13} />
          </button>
          <button
            type="button"
            className="shadow-soft inline-flex min-h-11 w-full items-center justify-center gap-3 rounded-lg border border-stone-200 bg-white/70 p-3 text-xs font-bold text-stone-700 transition-[transform,box-shadow,background] duration-200 hover:-translate-y-0.5 hover:bg-white active:translate-y-0"
            onClick={onInvoke}
          >
            <Icon name="external" size={13} /> Invoke in agent
          </button>
        </div>
        <div className="mt-3 text-center text-xs text-stone-500">
          Local preview · no payment or live execution
        </div>
      </div>
    </aside>
  );
};

const CapabilityDialog = ({
  tool,
  mode,
  onClose,
  onExecution,
}: {
  tool: Capability;
  mode: "sample" | "invoke";
  onClose: () => void;
  onExecution: (tool: Capability, input: string) => void;
}) => {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [url, setUrl] = useState("https://github.com/hive-labs/example");
  const [revision, setRevision] = useState("main");
  const [ran, setRan] = useState(false);
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState(false);
  const [validation, setValidation] = useState("");
  const [result, setResult] = useState<Record<string, unknown> | null>(null);
  const handoff = JSON.stringify(
    {
      capability: tool.id,
      input: { repository_url: url, revision },
      note: "Integration contract example. No live endpoint or payment is configured.",
      preview: true,
      quoted_price_usd: tool.price,
      version: "1.0",
    },
    null,
    2
  );
  useEffect(() => {
    const dialog = dialogRef.current;
    dialog?.showModal();
    return () => {
      dialog?.close();
    };
  }, []);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(
        mode === "invoke" ? handoff : JSON.stringify(result, null, 2)
      );
      setCopied(true);
      setCopyError(false);
    } catch {
      setCopyError(true);
    }
  };
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    try {
      const parsed = new URL(url);
      if (
        parsed.hostname !== "github.com" ||
        parsed.protocol !== "https:" ||
        parsed.pathname.split("/").filter(Boolean).length < 2
      ) {
        setValidation(
          "Use a public GitHub repository URL, such as https://github.com/owner/repo."
        );
        return;
      }
    } catch {
      setValidation("Enter a valid GitHub repository URL.");
      return;
    }
    if (!revision.trim()) {
      setValidation("Add a branch, tag, or commit revision.");
      return;
    }
    setValidation("");
    setRan(true);
    setCopied(false);
    setResult({
      input: { repository_url: url, revision },
      preview: true,
      sample_response: tool.output,
    });
    onExecution(tool, url);
  };
  return (
    <dialog
      ref={dialogRef}
      className="shadow-peek m-auto max-h-[calc(100dvh-50px)] w-[min(620px,100%)] overflow-y-auto rounded-3xl border border-white bg-stone-50 p-7 backdrop:bg-stone-900/25 backdrop:backdrop-blur-sm max-[760px]:rounded-xl max-[760px]:px-5 max-[760px]:py-6"
      onClose={onClose}
      aria-labelledby="dialog-title"
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <h2
            id="dialog-title"
            className="text-xl font-semibold tracking-tight max-[760px]:text-lg"
          >
            {mode === "sample"
              ? "Give it a little test drive."
              : "A handoff for your agent."}
          </h2>
          <p className="mt-2 mb-6 text-xs leading-relaxed text-stone-400">
            {tool.name} · v1.0 ·{" "}
            {mode === "sample"
              ? "Sample response preview"
              : "Integration contract preview"}
          </p>
        </div>
        <button
          className="inline-flex size-9 shrink-0 items-center justify-center rounded-lg border border-stone-200 bg-white hover:bg-stone-100"
          type="button"
          aria-label="Close dialog"
          onClick={onClose}
        >
          <Icon name="close" size={16} />
        </button>
      </div>
      <div className="mb-6 flex items-start gap-2.5 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3.5 text-xs leading-relaxed text-amber-800">
        <Icon name="help" size={16} />
        {mode === "sample"
          ? "This returns a fixed example response. Your repository is not fetched or analyzed, and no payment is made."
          : "Copy this example contract to discuss an integration with your agent. Hive’s live endpoint and payment connection are still being built."}
      </div>
      <form onSubmit={submit}>
        <label className="mb-5 flex flex-col gap-2 text-xs font-semibold text-stone-600">
          Repository URL
          <input
            type="url"
            required
            className="w-full rounded-md border border-stone-200 bg-white p-3 text-xs font-normal text-stone-700 focus:border-lime-700"
            value={url}
            onChange={(event) => {
              setUrl(event.target.value);
              setRan(false);
              setCopied(false);
            }}
          />
        </label>
        <label className="mb-5 flex flex-col gap-2 text-xs font-semibold text-stone-600">
          Revision
          <input
            type="text"
            required
            className="w-full rounded-md border border-stone-200 bg-white p-3 text-xs font-normal text-stone-700 focus:border-lime-700"
            value={revision}
            onChange={(event) => {
              setRevision(event.target.value);
              setRan(false);
              setCopied(false);
            }}
          />
          <span className="text-xs font-normal text-stone-400">
            A branch, tag, or commit — so the input stays explicit.
          </span>
        </label>
        {validation && (
          <p className="mb-4 text-xs font-normal text-amber-800" role="alert">
            {validation}
          </p>
        )}
        {mode === "sample" ? (
          <button
            type="submit"
            className="inline-flex min-h-12 items-center justify-center gap-3 rounded-xl border border-amber-300 bg-[linear-gradient(135deg,#ffe3a0,#f5ca69)] px-5 py-3 text-sm font-bold text-amber-900 transition-[transform,box-shadow,background] duration-200 hover:-translate-y-0.5 active:translate-y-0"
          >
            <Icon name="play" size={15} />
            {ran ? "Show sample again" : "Show sample response"}
          </button>
        ) : (
          <>
            <pre className="mx-0 my-4 rounded-xl border border-stone-200 bg-stone-100 p-4 text-xs leading-relaxed break-words whitespace-pre-wrap text-lime-800">
              {handoff}
            </pre>
            <button
              type="button"
              className="shadow-nav hover:shadow-nav inline-flex min-h-12 items-center justify-center gap-3 rounded-xl border border-transparent bg-stone-800 px-5 py-3 text-sm font-bold text-white transition-[transform,box-shadow,background] duration-200 hover:-translate-y-0.5 hover:bg-stone-700 active:translate-y-0"
              onClick={copy}
            >
              <Icon name={copied ? "check" : "copy"} size={15} />
              {copied ? "Copied to clipboard" : "Copy example contract"}
            </button>
          </>
        )}
      </form>
      {ran && mode === "sample" && (
        <>
          <div className="mt-6 flex items-center justify-between gap-2.5 text-xs text-lime-800">
            <span>
              <Icon
                name="check"
                size={13}
                style={{ display: "inline", marginRight: 5 }}
              />{" "}
              Example response · $0.00 charged
            </span>
            <button
              type="button"
              className="inline-flex size-9 items-center justify-center rounded-lg border border-stone-200 bg-white hover:bg-stone-100"
              onClick={copy}
              aria-label={copied ? "Response copied" : "Copy response"}
            >
              <Icon name={copied ? "check" : "copy"} size={14} />
            </button>
          </div>
          <pre className="mx-0 my-4 rounded-xl border border-stone-200 bg-stone-100 p-4 text-xs leading-relaxed break-words whitespace-pre-wrap text-lime-800">
            {JSON.stringify(result, null, 2)}
          </pre>
          <output className="text-xs font-normal text-stone-400">
            Added to this session’s sample execution history.
          </output>
        </>
      )}
      {copyError && (
        <output className="text-xs font-normal text-stone-400">
          Clipboard access is unavailable. Select and copy the example above.
        </output>
      )}
    </dialog>
  );
};

const getResults = (query: string, filter: string, sort: string) => {
  const searchTerms =
    query
      .toLowerCase()
      .match(/[a-z0-9]+/gu)
      ?.filter(
        (term) =>
          ![
            "a",
            "an",
            "the",
            "for",
            "my",
            "me",
            "with",
            "and",
            "to",
            "of",
            "i",
            "need",
            "find",
            "check",
          ].includes(term)
      ) ?? [];
  const scored = capabilities
    .map((item) => {
      const text =
        `${item.name} ${item.description} ${item.type} ${item.tags.join(" ")}`.toLowerCase();
      const matches = searchTerms.reduce(
        (score, term) => score + (text.includes(term) ? 1 : 0),
        0
      );
      return { item, matches };
    })
    .filter(
      ({ item, matches }) =>
        (filter === "All" || item.type === filterTypes[filter]) &&
        (searchTerms.length === 0 || matches > 0)
    );
  scored.sort((a, b) => {
    if (sort === "price") {
      return a.item.price - b.item.price;
    }
    if (sort === "speed") {
      return a.item.seconds - b.item.seconds;
    }
    return b.matches - a.matches;
  });
  const results = scored.map((result) => result.item);
  return results;
};

const Workspace = () => {
  const params = useSearchParams();
  const requestedView = params.get("view");
  const [view, setView] = useState<View>(
    isView(requestedView) ? requestedView : "discover"
  );
  const [selected, setSelected] = useState(
    capabilities.find((item) => item.id === params.get("tool")) ??
      firstCapability
  );
  const [query, setQuery] = useState(() => params.get("q") ?? "");
  const [filter, setFilter] = useState<string>("All");
  const [sort, setSort] = useState("relevance");
  const [modal, setModal] = useState<Modal>(null);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [executions, setExecutions] = useState<Execution[]>([]);
  const { drafts, workflow, budget, workspaceName, storageError } =
    usePreview();
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const handleKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === "k") {
        event.preventDefault();
        setView("discover");
        requestAnimationFrame(() => searchRef.current?.focus());
      }
    };
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, []);

  const navigate = (next: View) => {
    setView(next);
    setMobileOpen(false);
  };
  const results = getResults(query, filter, sort);
  const viewName =
    [...navigation, ...tabs].find((item) => item.id === view)?.label ??
    "Discover";
  const addExecution = (tool: Capability, input: string) =>
    setExecutions((current) => [
      {
        createdAt: new Date().toISOString(),
        id: crypto.randomUUID(),
        input,
        name: tool.name,
        output: tool.output,
      },
      ...current,
    ]);

  return (
    <div className="grid min-h-dvh grid-cols-[222px_minmax(0,1fr)] bg-stone-50 max-[1200px]:grid-cols-[195px_minmax(0,1fr)] max-[760px]:block min-[1550px]:grid-cols-[244px_minmax(0,1fr)]">
      <a
        className="fixed top-[-70px] left-5 z-50 rounded-xl bg-stone-800 px-4 py-3 text-white focus:top-3.5"
        href="#workspace-content"
      >
        Skip to workspace
      </a>
      {mobileOpen && (
        <button
          type="button"
          className="fixed inset-0 z-40 border-0 bg-stone-900/25"
          aria-label="Close navigation"
          onClick={() => setMobileOpen(false)}
        />
      )}
      <aside
        className={`max-[760px]:shadow-peek sticky top-0 z-40 flex h-dvh flex-col border-r border-stone-200 bg-white px-4 pt-8 pb-5 max-[1200px]:px-3 max-[760px]:fixed max-[760px]:top-0 max-[760px]:left-0 max-[760px]:w-56 max-[760px]:transition-transform max-[760px]:duration-200 ${
          mobileOpen
            ? "max-[760px]:translate-x-0"
            : "max-[760px]:-translate-x-full"
        }`}
        aria-label="Workspace navigation"
      >
        <button
          type="button"
          className="absolute top-7 right-2.5 hidden size-7 items-center justify-center rounded-lg border-0 bg-white hover:bg-stone-100 max-[760px]:flex"
          onClick={() => setMobileOpen(false)}
          aria-label="Close navigation"
        >
          <Icon name="close" size={16} />
        </button>
        <div className="flex items-center gap-2.5 px-4 pb-8">
          <Brand small />
          <span className="mt-1 rounded-sm border border-stone-200 px-1 py-1 text-xs leading-none font-semibold text-stone-500">
            BETA
          </span>
        </div>
        <button
          className="flex w-full items-center gap-2 rounded-xl border border-stone-200 bg-white px-2.5 py-3 text-left shadow-sm"
          type="button"
          onClick={() => navigate("settings")}
        >
          <span className="flex size-7 items-center justify-center rounded-lg bg-amber-50 text-amber-800">
            <Icon name="layers" size={15} />
          </span>
          <div className="flex flex-col gap-1">
            <strong className="max-w-28 overflow-hidden text-xs font-bold text-ellipsis whitespace-nowrap">
              {workspaceName}
            </strong>
            <small className="text-xs text-stone-500">Preview workspace</small>
          </div>
          <Icon className="ml-auto text-stone-400" name="down" size={12} />
        </button>
        <div className="px-3 pt-6 pb-3 text-xs font-bold tracking-widest text-stone-400">
          WORKSPACE
        </div>
        <nav className="flex flex-col gap-1">
          {navigation.map((item) => (
            <button
              type="button"
              className={`flex items-center gap-3 rounded-lg border px-3 py-3 text-left text-xs font-semibold ${
                view === item.id
                  ? "border-amber-200 bg-[linear-gradient(105deg,#fbefcd,#fcf6e5)] text-amber-800"
                  : "border-transparent bg-transparent text-stone-600 hover:bg-stone-100 hover:text-stone-600"
              }`}
              key={item.id}
              onClick={() => navigate(item.id)}
              aria-current={view === item.id ? "page" : undefined}
            >
              <Icon name={item.icon} size={17} />
              {item.label}
              {item.id === "discover" && (
                <span className="ml-auto text-xs text-amber-800/70">⌘ K</span>
              )}
            </button>
          ))}
        </nav>
        <div className="mt-auto pt-10">
          <div className="mb-3.5 rounded-xl border border-stone-200 bg-white p-4">
            <div className="flex items-center justify-between text-xs text-stone-500">
              <span className="flex items-center gap-1.5">
                <Icon name="wallet" size={12} /> Monthly budget
              </span>
              <button
                type="button"
                className="inline-flex items-center gap-2.5 border-0 bg-none p-0.5 text-xs font-bold text-stone-500 hover:text-stone-500"
                aria-label="Edit budget"
                onClick={() => navigate("billing")}
              >
                <Icon name="chevron" size={10} />
              </button>
            </div>
            <strong className="mt-3 inline-block text-sm font-semibold">
              {money(42.17)}{" "}
              <small className="text-xs font-normal text-stone-400">
                / {money(budget)}
              </small>
            </strong>
            <div className="mt-3 h-1 overflow-hidden rounded-md bg-stone-200">
              <span
                className="block h-full w-[42.17%] rounded-[inherit] bg-lime-300"
                style={{
                  width: `${budget > 0 ? Math.min((42.17 / budget) * 100, 100) : 100}%`,
                }}
              />
            </div>
            <span className="mt-2 block text-xs text-stone-400">
              Illustrative spend · no live charges
            </span>
          </div>
          <Link
            href="/#docs"
            className="flex items-center gap-2 px-3 pt-2.5 pb-4 text-xs text-stone-500"
          >
            <Icon name="help" size={15} /> A little guidance{" "}
            <Icon name="external" size={11} />
          </Link>
          <button
            className="flex w-full items-center gap-2 border-0 border-t border-stone-200 bg-transparent px-1.5 pt-4 pb-0 text-left"
            type="button"
            onClick={() => navigate("settings")}
          >
            <span className="inline-flex size-8 items-center justify-center rounded-full bg-stone-200 text-xs text-stone-600">
              Y
            </span>
            <div className="flex flex-col gap-1">
              <strong className="text-xs font-semibold">Your workspace</strong>
              <small className="text-xs text-stone-500">
                Make yourself at home
              </small>
            </div>
            <Icon className="ml-auto text-stone-500" name="down" size={12} />
          </button>
        </div>
      </aside>
      <div className="min-w-0">
        <header className="flex min-h-16 items-center justify-between gap-4 border-b border-stone-200 bg-white/80 px-8 max-[760px]:px-5 max-[380px]:px-4">
          <div className="flex items-center gap-3 text-xs text-stone-500 max-[760px]:gap-2 max-[380px]:text-xs">
            <button
              type="button"
              className="hidden size-9 items-center justify-center rounded-lg border border-stone-200 bg-white hover:bg-stone-100 max-[760px]:flex"
              aria-label="Open navigation"
              onClick={() => setMobileOpen(true)}
            >
              <Icon name="menu" size={16} />
            </button>
            <Icon name="layers" size={14} />
            <span>Workspace</span>
            <Icon name="chevron" size={10} />
            <strong className="font-semibold text-stone-700">{viewName}</strong>
          </div>
          <div className="flex items-center gap-5 text-xs text-stone-500 max-[760px]:gap-2.5">
            <span className="rounded-md border border-stone-200 bg-stone-100 px-2 py-1 text-xs text-stone-500">
              Preview workspace
            </span>
            <Link
              href="/#docs"
              className="inline-flex items-center gap-1 max-[760px]:hidden"
            >
              Docs <Icon name="external" size={11} />
            </Link>
            <Link
              href="/"
              className="inline-flex items-center gap-1 max-[760px]:hidden"
              aria-label="Back to home"
            >
              <Icon name="logout" size={15} />
            </Link>
          </div>
        </header>
        <nav
          className="flex gap-6 border-b border-stone-200 bg-white/80 px-8 max-[760px]:gap-5 max-[760px]:px-5 max-[380px]:gap-4"
          aria-label="Capability lifecycle"
        >
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => navigate(tab.id)}
              className={`inline-flex items-center gap-2 border-0 border-b-2 border-b-transparent pt-4 pb-3.5 text-xs font-semibold ${
                view === tab.id
                  ? "border-b-lime-700 text-lime-800"
                  : "bg-none text-stone-500"
              }`}
              aria-current={view === tab.id ? "page" : undefined}
            >
              <Icon name={tab.icon} size={13} />
              {tab.label}
            </button>
          ))}
        </nav>
        <main id="workspace-content">
          {storageError && (
            <output className="my-6 flex items-start gap-2.5 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3.5 text-xs leading-relaxed text-amber-800 [&>svg]:mt-px [&>svg]:shrink-0">
              Browser storage is unavailable. Your changes will stay in this
              session.
            </output>
          )}
          {view === "discover" && (
            <div className="mx-auto my-0 grid max-w-7xl grid-cols-[minmax(340px,1fr)_335px] gap-6 p-8 max-[1200px]:grid-cols-[minmax(300px,1fr)_300px] max-[1200px]:gap-5 max-[1200px]:px-6 max-[1200px]:py-6 max-[1000px]:grid-cols-[minmax(0,1fr)] max-[760px]:grid-cols-[minmax(0,1fr)] max-[760px]:gap-6 max-[760px]:px-4 max-[760px]:py-6 min-[1550px]:grid-cols-[minmax(400px,1fr)_370px] min-[1550px]:gap-8 min-[1550px]:p-10">
              <section className="min-w-0" aria-label="Discover capabilities">
                <div className="mb-3.5 flex items-center gap-2 text-xs text-lime-800 max-[760px]:pl-1">
                  <span className="inline-block size-1.5 shrink-0 rounded-full bg-lime-700" />{" "}
                  GOOD TOOLS. LESS GROUNDWORK.
                </div>
                <div className="flex items-start justify-between gap-4 max-[760px]:pl-1">
                  <div>
                    <h1 className="text-3xl leading-tight font-semibold tracking-tight max-[1000px]:text-3xl max-[760px]:text-2xl">
                      A head start
                      <br />
                      <span className="text-lime-700">for every task.</span>
                    </h1>
                    <p className="mt-3 text-xs leading-relaxed text-stone-500 max-[760px]:max-w-60 max-[760px]:text-xs">
                      Find the right capability. Give your agent a little more
                      reach.
                    </p>
                  </div>
                  <Image
                    src="/bee-left.png"
                    alt=""
                    width={325}
                    height={304}
                    className="mt-[-2px] mr-2 h-auto w-16 rotate-[9deg] max-[1200px]:hidden max-[1000px]:block max-[760px]:w-16"
                  />
                </div>
                <search>
                  <form
                    className="mt-6 flex items-center gap-2.5 rounded-xl border border-stone-200 bg-white px-2 py-2 pl-4 text-stone-400 shadow-sm focus-within:border-lime-700 focus-within:ring-4 focus-within:ring-lime-100 max-[760px]:mt-6 max-[760px]:pl-3"
                    onSubmit={(event) => {
                      event.preventDefault();
                      if (results[0]) {
                        setSelected(results[0]);
                      }
                    }}
                  >
                    <Icon name="search" size={18} />
                    <input
                      ref={searchRef}
                      aria-label="Search capabilities"
                      placeholder="What does your agent need to do?"
                      className="w-full min-w-0 border-0 bg-transparent text-xs text-stone-700 outline-none placeholder:text-stone-400 max-[760px]:text-xs"
                      value={query}
                      onChange={(event) => setQuery(event.target.value)}
                    />
                    {query && (
                      <button
                        type="button"
                        className="inline-flex size-8 items-center justify-center rounded-lg border border-stone-200 bg-white hover:bg-stone-100"
                        onClick={() => {
                          setQuery("");
                          searchRef.current?.focus();
                        }}
                        aria-label="Clear search"
                      >
                        <Icon name="close" size={13} />
                      </button>
                    )}
                    <button
                      className="inline-flex size-8 shrink-0 items-center justify-center rounded-md border border-stone-200 bg-stone-100 text-lime-800"
                      type="submit"
                      aria-label="Find capabilities"
                    >
                      <Icon name="arrow" size={16} />
                    </button>
                  </form>
                </search>
                <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-stone-400">
                  <span>Try a little something:</span>
                  {[
                    "Audit a GitHub repo",
                    "Extract documentation",
                    "Check licenses",
                  ].map((suggestion) => (
                    <button
                      type="button"
                      key={suggestion}
                      className="border-0 border-b border-dotted border-stone-300 bg-none p-0 text-xs text-lime-800"
                      onClick={() => {
                        setQuery(suggestion);
                        setFilter("All");
                      }}
                    >
                      {suggestion}
                    </button>
                  ))}
                </div>
                <div className="mt-7 flex flex-wrap items-center justify-between gap-2.5 max-[760px]:mt-6 max-[760px]:gap-3.5">
                  <div
                    className="flex flex-wrap gap-1"
                    aria-label="Capability type"
                  >
                    {filters.map((type) => (
                      <button
                        type="button"
                        key={type}
                        className={`rounded-lg border px-2.5 py-1.5 text-xs font-semibold max-[760px]:px-2 max-[760px]:py-1.5 max-[760px]:text-xs ${
                          filter === type
                            ? "border-stone-200 bg-white text-stone-700 shadow-sm"
                            : "border-transparent bg-transparent text-stone-500"
                        }`}
                        onClick={() => setFilter(type)}
                        aria-pressed={filter === type}
                      >
                        {type}
                      </button>
                    ))}
                  </div>
                  <label className="flex items-center gap-1 text-xs text-stone-400">
                    Sort by{" "}
                    <select
                      value={sort}
                      onChange={(event) => setSort(event.target.value)}
                      aria-label="Sort capabilities"
                      className="max-w-24 border-0 bg-none py-1 text-xs text-stone-600"
                    >
                      <option value="relevance">Relevance</option>
                      <option value="price">Lowest price</option>
                      <option value="speed">Fastest</option>
                    </select>
                  </label>
                </div>
                <div className="mx-0 mt-5 mb-3 flex justify-between text-xs text-stone-500 max-[760px]:mt-4 max-[760px]:text-xs">
                  <span>
                    <strong className="font-medium text-stone-600">
                      {results.length}
                    </strong>{" "}
                    capabilities to build on
                  </span>
                  <span>Example listings · illustrative prices</span>
                </div>
                <div className="flex flex-col gap-1.5">
                  {results.map((tool) => (
                    <button
                      className={`flex w-full items-start gap-3 rounded-xl border px-3 py-4 text-left transition-[border-color,background] duration-150 max-[1200px]:gap-2.5 max-[1200px]:px-2.5 max-[1000px]:p-4 max-[760px]:gap-2.5 max-[760px]:px-2.5 max-[760px]:py-4 min-[1550px]:px-4 min-[1550px]:py-5 ${
                        selected.id === tool.id
                          ? "border-amber-200 bg-[linear-gradient(110deg,#fffaf0,#fffef8)] shadow-sm"
                          : "border-transparent bg-transparent hover:bg-white/70"
                      }`}
                      type="button"
                      key={tool.id}
                      onClick={() => setSelected(tool)}
                      aria-pressed={selected.id === tool.id}
                      aria-label={`${tool.name}, ${money(tool.price)} per call. View details`}
                    >
                      <span
                        className={`${toolIconBase} mt-px size-9 rounded-lg max-[760px]:size-9`}
                      >
                        <Icon name={tool.icon} size={18} />
                      </span>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <strong className="text-xs font-bold text-stone-700">
                            {tool.name}
                          </strong>
                        </div>
                        <p className="mt-1.5 max-w-full overflow-hidden text-xs leading-relaxed text-ellipsis whitespace-nowrap text-stone-500">
                          {tool.description}
                        </p>
                        <div className="mt-2 flex flex-wrap gap-1">
                          <span
                            className={`${tagPillBase} ${
                              selected.id === tool.id ? tagPillSelectedBase : ""
                            }`}
                          >
                            {tool.type}
                          </span>
                          {tool.tags.map((tag) => (
                            <span
                              className={`${tagPillBase} ${
                                selected.id === tool.id
                                  ? tagPillSelectedBase
                                  : ""
                              }`}
                              key={tag}
                            >
                              {tag}
                            </span>
                          ))}
                        </div>
                      </div>
                      <div className="flex shrink-0 flex-col gap-2 pt-0 text-right">
                        <strong className="text-xs font-semibold text-stone-700">
                          {money(tool.price)}
                        </strong>
                        <small className="text-xs text-stone-400">
                          ~{tool.seconds}s
                        </small>
                        <Verification verified={tool.verified} />
                      </div>
                      <Icon
                        className="mt-3 text-stone-300 max-[760px]:ml-[-4px]"
                        name="chevron"
                        size={13}
                      />
                    </button>
                  ))}
                </div>
                {results.length === 0 && (
                  <div className="px-5 py-12 text-center text-stone-400">
                    <Icon
                      className="mx-auto mt-0 mr-auto mb-4 ml-auto"
                      name="search"
                      size={27}
                    />
                    <h2 className="mb-2.5 text-lg text-stone-600">
                      No match just yet.
                    </h2>
                    <p className="mb-5 text-xs leading-relaxed">
                      Try a broader search or another capability type.
                    </p>
                    <button
                      type="button"
                      className="shadow-soft inline-flex min-h-10 items-center justify-center gap-3 rounded-xl border border-stone-200 bg-white/70 px-4 py-2.5 text-xs font-bold text-stone-700 transition-[transform,box-shadow,background] duration-200 hover:-translate-y-0.5 hover:bg-white active:translate-y-0"
                      onClick={() => {
                        setQuery("");
                        setFilter("All");
                      }}
                    >
                      Show all capabilities
                    </button>
                  </div>
                )}
                <p className="mt-5 flex items-center justify-center gap-1.5 text-center text-xs text-stone-500">
                  <Icon name="layers" size={12} /> Built by someone. Ready to
                  become part of something.
                </p>
              </section>
              <ToolDetail
                key={selected.id}
                tool={selected}
                onSample={() => setModal("sample")}
                onInvoke={() => setModal("invoke")}
                onSelect={(tool) => setSelected(tool)}
              />
            </div>
          )}
          {view === "compose" && (
            <Composer
              workflow={workflow}
              onChange={(value) => updatePreview({ workflow: value })}
            />
          )}
          {view === "publish" && (
            <Publisher
              onSave={(draft) =>
                updatePreview((current) => ({
                  drafts: [draft, ...current.drafts],
                }))
              }
            />
          )}
          {view === "tools" && (
            <MyTools
              drafts={drafts}
              onPublish={() => navigate("publish")}
              onDelete={(id) =>
                updatePreview((current) => ({
                  drafts: current.drafts.filter((draft) => draft.id !== id),
                }))
              }
            />
          )}
          {view === "executions" && (
            <Executions
              executions={executions}
              onExplore={() => navigate("discover")}
            />
          )}
          {view === "billing" && (
            <Billing
              budget={budget}
              onSave={(value) => updatePreview({ budget: value })}
            />
          )}
          {view === "settings" && (
            <Settings
              name={workspaceName}
              onSave={(value) => updatePreview({ workspaceName: value })}
            />
          )}
          {view === "use" && (
            <section className="mx-auto max-w-5xl px-9 pt-10 pb-16 max-[760px]:px-5 max-[760px]:py-8">
              <span className="mb-3.5 flex items-center gap-2 text-xs text-lime-800">
                FROM DISCOVERY TO DOING
              </span>
              <h1 className="text-3xl font-semibold tracking-tight max-[760px]:text-2xl">
                A small tool. A useful next step.
              </h1>
              <p className="mt-3 text-xs leading-relaxed text-stone-500">
                Try a capability’s sample response, then inspect the contract
                your agent would use.
              </p>
              <div className="my-6 flex items-start gap-2.5 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3.5 text-xs leading-relaxed text-amber-800">
                <Icon className="mt-px shrink-0" name="help" size={16} />
                This workspace uses local examples. Live execution, agent
                connections, and payment are not enabled.
              </div>
              <div className="mt-6 rounded-2xl border border-stone-200 bg-white p-6 shadow-sm max-[760px]:p-5">
                <h2 className="mb-5 text-lg font-semibold tracking-tight">
                  Your selected capability
                </h2>
                <div className="flex items-center gap-3 border-b border-stone-200 pt-0 pb-4">
                  <span
                    className={`${toolIconBase} ${toolTint(selected.color)}`}
                  >
                    <Icon name={selected.icon} />
                  </span>
                  <div className="flex min-w-0 flex-col gap-1.5">
                    <strong className="text-xs font-semibold">
                      {selected.name}
                    </strong>
                    <small className="text-xs break-words text-stone-500">
                      {selected.type} · {money(selected.price)} per call ·
                      illustrative price
                    </small>
                  </div>
                </div>
                <p
                  className="text-xs leading-relaxed text-stone-500"
                  style={{ marginBottom: 20, marginTop: 18 }}
                >
                  {selected.description}
                </p>
                <div className="flex flex-wrap items-center gap-3">
                  <button
                    type="button"
                    className="inline-flex min-h-12 items-center justify-center gap-3 rounded-xl border border-amber-300 bg-[linear-gradient(135deg,#ffe3a0,#f5ca69)] px-5 py-3 text-sm font-bold text-amber-900 transition-[transform,box-shadow,background] duration-200 hover:-translate-y-0.5 active:translate-y-0"
                    onClick={() => setModal("sample")}
                  >
                    <Icon name="play" size={15} /> Try sample input
                  </button>
                  <button
                    type="button"
                    className="shadow-soft inline-flex min-h-12 items-center justify-center gap-3 rounded-xl border border-stone-200 bg-white/70 px-5 py-3 text-sm font-bold text-stone-700 transition-[transform,box-shadow,background] duration-200 hover:-translate-y-0.5 hover:bg-white active:translate-y-0"
                    onClick={() => setModal("invoke")}
                  >
                    <Icon name="code" size={15} /> View agent handoff
                  </button>
                </div>
              </div>
            </section>
          )}
        </main>
      </div>
      {modal && (
        <CapabilityDialog
          tool={selected}
          mode={modal}
          onClose={() => setModal(null)}
          onExecution={addExecution}
        />
      )}
    </div>
  );
};

export default Workspace;
