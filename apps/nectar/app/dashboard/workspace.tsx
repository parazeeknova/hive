"use client";
import {
  useInfiniteQuery,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import type { InfiniteData } from "@tanstack/react-query";
import Image from "next/image";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";

import { Brand } from "../../components/brand";
import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";
import type { Profile } from "../../lib/account";
import { capabilityDisplayName } from "../../lib/capability-name";
import type {
  CatalogMetadata,
  Activity,
  Credential,
  RegistryCapability,
  Runtime,
  Sale,
} from "../../lib/hiveway";
import {
  atomicMoney,
  errorMessage,
  hiveway,
  runtimeLabels,
  shortId,
} from "../../lib/hiveway";
import { useHiveSession } from "../../lib/providers";
import AccountPanel from "./account-panel";
import type { AccountTab } from "./account-panel";
import { SidebarBalance } from "./account-payments";
import { PublisherSummary } from "./publisher";
import {
  ActivityPanel,
  EmptyState,
  Json,
  CapabilityActionDialog,
  SalesPanel,
  SessionDialog,
} from "./workspace-pages";

import "./workspace.css";

type View = "discover" | "executions" | "billing" | "settings";
const navigation: { id: View; label: string; icon: IconName; group: string }[] =
  [
    { group: "Workspace", icon: "search", id: "discover", label: "Discover" },
    {
      group: "Workspace",
      icon: "activity",
      id: "executions",
      label: "Activity",
    },
    { group: "Account", icon: "wallet", id: "billing", label: "Sales" },
    {
      group: "Account",
      icon: "settings",
      id: "settings",
      label: "Account settings",
    },
  ];
const headings: Record<View, { title: string; description: string }> = {
  billing: {
    description: "Direct creator payments, with a record of every sale.",
    title: "Let your work earn.",
  },
  discover: {
    description: "Useful capabilities. Ready for your next task.",
    title: "Find your head start.",
  },
  executions: {
    description: "Invocations, payment states, and results in one place.",
    title: "Follow the work.",
  },
  settings: {
    description: "Your profile, agent balance, and marketplace capabilities.",
    title: "Account settings.",
  },
};

const Inspector = ({
  tool,
  catalog,
  token,
  onConnect,
  onRefresh,
  onSelect,
  onClose,
}: {
  tool: RegistryCapability;
  catalog: RegistryCapability[];
  token: string;
  onConnect: () => void;
  onRefresh: () => void;
  onSelect: (id: string) => void;
  onClose: () => void;
}) => {
  let outputDescription =
    "Report fields can include repository_url, revision, manifests, technologies, dependencies, and analysis_limit_reached. The settled result is the source of truth.";
  let limitationDescription =
    "Analysis describes declared repository manifests and may reach runtime analysis limits.";
  if (tool.runtime === "wasm") {
    outputDescription =
      "Output must match the published artifact output schema.";
    limitationDescription =
      "Execution is bounded by fuel, memory, input/output size, and schema validation.";
  } else if (tool.runtime === "workflow") {
    outputDescription =
      "Output contains repository identity and a steps array with each capability, version, runtime, and result.";
    limitationDescription =
      "Every step uses the same repository input. Conditional steps and nested workflows are not supported.";
  }
  const artifactId = tool.definition.artifact_id;
  const artifact = useQuery({
    enabled: Boolean(
      token && tool.runtime === "wasm" && typeof artifactId === "string"
    ),
    queryFn: ({ signal }) =>
      hiveway<{ input_schema: unknown; output_schema: unknown }>(
        `/v1/artifacts/${String(artifactId)}`,
        token,
        { signal }
      ),
    queryKey: ["hive", token, "artifact", artifactId],
    staleTime: Infinity,
  });
  const [action, setAction] = useState<"sample" | "invoke" | null>(null);
  const steps = Array.isArray(tool.definition.steps)
    ? tool.definition.steps.filter((id): id is string => typeof id === "string")
    : [];
  return (
    <>
      <div className="ws-author-row">
        <span className="ws-author-name">Capability details</span>
        <button
          className="ws-icon-button ws-detail-close"
          type="button"
          onClick={onClose}
          aria-label="Close capability details"
        >
          <Icon name="close" size={16} />
        </button>
      </div>
      <div className="ws-tool-title">
        <h2>{tool.name}</h2>
      </div>
      <p className="ws-capability-description">{tool.description}</p>
      <PublisherSummary metadata={tool.metadata} />
      <div className="ws-capability-tags">
        <span>{runtimeLabels[tool.runtime]}</span>
        <span>{tool.runtime === "wasm" ? "Sandboxed" : "GitHub"}</span>
      </div>
      <div className="ws-inspector-price">
        <strong>
          {atomicMoney(tool.price)} <small>USDC / call</small>
        </strong>
      </div>
      <div className="ws-detail-sections">
        <details className="ws-detail-section">
          <summary>
            <span>Overview</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body">
            <p>{tool.description}</p>
            <p>
              Runtime: <code>{tool.runtime}</code>. Version {tool.version}.
            </p>
          </div>
        </details>
        <details className="ws-detail-section">
          <summary>
            <span>Input / Output Schema</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body">
            <p>
              {tool.runtime === "wasm" ? "Artifact schemas" : "Required input"}
            </p>
            {artifact.error && (
              <p role="alert">{errorMessage(artifact.error)}</p>
            )}
            <Json
              value={
                artifact.data ??
                (tool.runtime === "wasm"
                  ? {
                      status: artifact.isFetching
                        ? "Loading artifact schemas"
                        : "Artifact schemas unavailable",
                    }
                  : {
                      properties: {
                        repository_url: {
                          description: "Public GitHub repository URL",
                          format: "uri",
                          type: "string",
                        },
                        revision: {
                          description: "Full immutable commit SHA",
                          pattern: "^[a-fA-F0-9]{40}$",
                          type: "string",
                        },
                      },
                      required: ["repository_url", "revision"],
                      type: "object",
                    })
              }
            />
            <p>{outputDescription}</p>
          </div>
        </details>
        <details className="ws-detail-section">
          <summary>
            <span>Sample Response</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body">
            <p>
              A sample response is not provided by the registry yet. Your actual
              settled output is available in Activity after execution.
            </p>
          </div>
        </details>
        <details className="ws-detail-section" open={steps.length > 0}>
          <summary>
            <span>
              Dependencies <small>({steps.length})</small>
            </span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body">
            {steps.length > 0 ? (
              <div className="ws-dependency-tree">
                {steps.map((id) => {
                  const dependency = catalog.find((item) => item.id === id);
                  return (
                    <button
                      type="button"
                      key={id}
                      disabled={!dependency}
                      onClick={() => onSelect(id)}
                    >
                      <span>{dependency?.name ?? shortId(id)}</span>
                      {dependency && (
                        <small>{atomicMoney(dependency.price)} USDC</small>
                      )}
                    </button>
                  );
                })}
              </div>
            ) : (
              <p>
                This capability runs independently, with no bundled
                capabilities.
              </p>
            )}
          </div>
        </details>
        <details className="ws-detail-section">
          <summary>
            <span>Pricing</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body">
            <p>
              {atomicMoney(tool.price)} USDC per invocation. Review the returned
              quote’s payment requirements before authorizing payment.
            </p>
            <p>
              A quote reserves session budget for 120 seconds. Quoting alone
              does not submit a payment.
            </p>
          </div>
        </details>
        <details className="ws-detail-section">
          <summary>
            <span>Limitations</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body">
            <p>
              {tool.runtime === "wasm"
                ? "Pure JSON-in/JSON-out Wasm. No filesystem, network, clock, or environment access."
                : "Public GitHub repositories only. Supply a full 40-character commit SHA; branches, tags, and private repositories are not supported."}
            </p>
            <p>{limitationDescription}</p>
          </div>
        </details>
      </div>
      <div className="ws-capability-actions">
        <button
          type="button"
          className="ws-button ws-primary"
          onClick={() => setAction("sample")}
        >
          <Icon name="play" size={16} />
          Test with sample input
        </button>
        <button
          type="button"
          className="ws-button"
          onClick={() => setAction("invoke")}
        >
          <Icon name="external" size={16} />
          Invoke in agent
        </button>
      </div>
      {action && (
        <CapabilityActionDialog
          mode={action}
          tool={tool}
          token={token}
          onConnect={onConnect}
          onRefresh={onRefresh}
          onClose={() => setAction(null)}
        />
      )}
    </>
  );
};

// View routing keeps navigation and selection in one state owner.
// eslint-disable-next-line complexity
const Workspace = () => {
  const params = useSearchParams();
  const initial = params.get("view");
  const [view, setView] = useState<View>(() => {
    if (navigation.some((item) => item.id === initial)) {
      return initial as View;
    }
    if (["tools", "compose", "publish", "account"].includes(initial ?? "")) {
      return "settings";
    }
    return "discover";
  });
  const [accountTab, setAccountTab] = useState<AccountTab>(() => {
    const tab = params.get("tab");
    return tab === "balance" || tab === "capabilities" || tab === "api-keys"
      ? tab
      : "profile";
  });
  const [query, setQuery] = useState(params.get("q") ?? "");
  const [filter, setFilter] = useState("all");
  const [sort, setSort] = useState("newest");
  const [selectedId, setSelectedId] = useState(params.get("tool") ?? "");
  const [detailOpen, setDetailOpen] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [connectOpen, setConnectOpen] = useState(false);
  const session = useHiveSession();
  const { token, setToken, setCredential, sessionKey } = session;
  const client = useQueryClient();
  const me = useQuery({
    enabled: Boolean(token),
    queryFn: ({ signal }) =>
      hiveway<Credential>("/v1/credentials/me", token, { signal }),
    queryKey: ["hive", sessionKey, "credential"],
  });
  const credential = me.data ?? session.credential;
  const accountProfile = useQuery({
    queryFn: ({ signal }) =>
      hiveway<Profile>("/v1/account", "session", { signal }),
    queryKey: ["hive-account", "profile"],
  });

  const history = useInfiniteQuery<
    Activity,
    Error,
    InfiniteData<Activity>,
    readonly unknown[],
    string
  >({
    enabled: Boolean(token && credential?.scopes.includes("receipts:read")),
    getNextPageParam: (page) =>
      page.next_cursor
        ? new URLSearchParams({
            before_id: page.next_cursor.before_id,
            before_time: String(page.next_cursor.before_time),
          }).toString()
        : undefined,
    initialPageParam: "",
    queryFn: ({ signal, pageParam }) =>
      hiveway<Activity>(
        `/v1/activity${pageParam ? `?${pageParam}` : ""}`,
        token,
        { signal }
      ),
    queryKey: ["hive", sessionKey, "activity"],
    refetchInterval: view === "executions" ? 3000 : 15_000,
  });
  const plan = useQuery({
    queryFn: ({ signal }) =>
      hiveway<{ tier: string; build_credits: { available: number } | null }>(
        "/v1/account/plan",
        "session",
        { signal }
      ),
    queryKey: ["hive-account", "plan"],
    refetchInterval: 15_000,
  });
  const earnings = useQuery({
    enabled: Boolean(token && credential?.scopes.includes("sales:read")),
    queryFn: ({ signal }) => hiveway<Sale[]>("/v1/sales", token, { signal }),
    queryKey: ["hive", sessionKey, "sales"],
  });
  const ownedView = view === "settings";
  const [debouncedQuery, setDebouncedQuery] = useState(query);
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedQuery(query), 200);
    return () => clearTimeout(timer);
  }, [query]);
  const registry = useInfiniteQuery<
    { items: RegistryCapability[]; next_cursor: string | null },
    Error,
    InfiniteData<{
      items: RegistryCapability[];
      next_cursor: string | null;
    }>,
    readonly unknown[],
    string
  >({
    enabled: Boolean(token && credential?.scopes.includes("capabilities:read")),
    getNextPageParam: (page) => page.next_cursor ?? undefined,
    initialPageParam: "",
    placeholderData: (previous, previousQuery) =>
      previousQuery?.queryKey[1] === sessionKey &&
      previousQuery.queryKey[3] === ownedView
        ? previous
        : undefined,
    queryFn: ({ pageParam, signal }) => {
      const search = new URLSearchParams({
        limit: "50",
        q: ownedView ? "" : debouncedQuery.trim(),
      });
      if (ownedView) {
        search.set("mine", "true");
        search.set("include_inactive", "true");
      }
      if (!ownedView && filter !== "all") {
        search.set("runtime", filter);
      }
      if (pageParam) {
        search.set("cursor", pageParam);
      }
      return hiveway<{
        items: RegistryCapability[];
        next_cursor: string | null;
      }>(`/v1/capabilities/page?${search}`, token, { signal });
    },
    queryKey: [
      "hive",
      sessionKey,
      "catalog",
      ownedView,
      ownedView ? "" : debouncedQuery,
      filter,
    ],
  });
  const rawCatalog = token
    ? (registry.data?.pages.flatMap((page) => page.items) ?? [])
    : [];
  const metadataIds = rawCatalog.map((tool) => tool.id);
  const metadata = useQuery({
    enabled: Boolean(token && metadataIds.length),
    queryFn: async ({ signal }) => {
      const batches: string[][] = [];
      for (let index = 0; index < metadataIds.length; index += 100) {
        batches.push(metadataIds.slice(index, index + 100));
      }
      const pages = await Promise.all(
        batches.map((ids) =>
          hiveway<CatalogMetadata[]>("/v1/catalog/metadata", token, {
            body: JSON.stringify({ ids }),
            method: "POST",
            signal,
          })
        )
      );
      return pages.flat();
    },
    queryKey: ["hive", sessionKey, "catalog-metadata", metadataIds],
    refetchInterval: 10_000,
  });
  const byId = new Map(
    metadata.data?.map((item) => [item.capability_id, item])
  );
  const catalog = rawCatalog.map((tool) => ({
    ...tool,
    metadata: byId.get(tool.id),
    name: capabilityDisplayName(tool.name),
  }));
  const latestActivity = history.data?.pages[0];
  const activity = latestActivity
    ? {
        ...latestActivity,
        invocations:
          history.data?.pages.flatMap((page) => page.invocations) ?? [],
      }
    : null;
  const sales = earnings.data ?? [];
  const loading = registry.isFetching;
  const failure = me.error ?? registry.error ?? history.error ?? earnings.error;
  const error = failure ? errorMessage(failure) : "";
  const search = useRef<HTMLInputElement>(null);
  const inspector = useRef<HTMLElement>(null);
  const refreshData = useCallback(() => {
    void client.invalidateQueries({ queryKey: ["hive", sessionKey] });
  }, [client, sessionKey]);
  const disconnect = () => {
    setToken("session");
    setCredential(null);
  };
  const navigate = (next: View, tab?: AccountTab) => {
    if (tab) {
      setAccountTab(tab);
    }
    setView(next);
    setMobileOpen(false);
    setDetailOpen(false);
    setQuery("");
    setFilter("all");
    const url = new URL(window.location.href);
    url.searchParams.set("view", next);
    if (next === "settings") {
      url.searchParams.set("tab", tab ?? accountTab);
    } else {
      url.searchParams.delete("tab");
    }
    url.searchParams.delete("q");
    window.history.replaceState(null, "", url);
  };

  useEffect(() => {
    if (!mobileOpen) {
      return;
    }
    const previous = document.activeElement;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    document.querySelector<HTMLButtonElement>(".ws-menu-close")?.focus();
    const media = window.matchMedia("(max-width: 760px)");
    const handleResize = () => {
      if (!media.matches) {
        setMobileOpen(false);
      }
    };
    media.addEventListener("change", handleResize);
    return () => {
      document.body.style.overflow = previousOverflow;
      media.removeEventListener("change", handleResize);
      if (previous instanceof HTMLElement && previous.isConnected) {
        previous.focus();
      }
    };
  }, [mobileOpen]);
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === "k") {
        event.preventDefault();
        setView("discover");
        requestAnimationFrame(() => search.current?.focus());
      }
      if (event.key === "Escape") {
        setMobileOpen(false);
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, []);

  const visibleCatalog = catalog;
  let results = visibleCatalog.filter(
    (item) => item.active && (filter === "all" || item.runtime === filter)
  );
  if (!token) {
    results = results.filter((item) =>
      `${item.name} ${item.description}`
        .toLowerCase()
        .includes(query.toLowerCase())
    );
  }
  if (sort === "price") {
    results = results.toSorted((a, b) => {
      if (BigInt(a.price) < BigInt(b.price)) {
        return -1;
      }
      if (BigInt(a.price) > BigInt(b.price)) {
        return 1;
      }
      return 0;
    });
  }
  if (sort === "name") {
    results = results.toSorted((a, b) => a.name.localeCompare(b.name));
  }
  const selected = results.find((item) => item.id === selectedId) ?? results[0];
  const openDetail = (id: string) => {
    setSelectedId(id);
    setDetailOpen(true);
    requestAnimationFrame(() => {
      if (window.matchMedia("(max-width: 1100px)").matches) {
        inspector.current?.scrollIntoView({
          behavior: window.matchMedia("(prefers-reduced-motion: reduce)")
            .matches
            ? "instant"
            : "smooth",
          block: "start",
        });
      }
    });
  };
  const catalogView = view === "discover";
  const head = headings[view];
  return (
    <div className={`ws-app ${view === "discover" ? "ws-discover-view" : ""}`}>
      <a className="ws-skip" href="#workspace-content">
        Skip to workspace
      </a>
      {mobileOpen && (
        <button
          type="button"
          className="ws-nav-backdrop"
          aria-label="Close navigation"
          onClick={() => setMobileOpen(false)}
        />
      )}
      <aside
        className={`ws-sidebar ${mobileOpen ? "is-open" : ""}`}
        aria-label="Workspace navigation"
      >
        <div className="ws-brand">
          <Brand small />
          <span className="ws-beta">BETA</span>
          <button
            type="button"
            className="ws-icon-button ws-menu-close"
            aria-label="Close navigation"
            onClick={() => setMobileOpen(false)}
          >
            <Icon name="close" size={18} />
          </button>
        </div>
        <nav>
          {["Workspace", "Account"].map((group) => (
            <div className="ws-nav-group" key={group}>
              <span className="ws-overline">{group}</span>
              {navigation
                .filter((item) => item.group === group)
                .map((item) => (
                  <button
                    type="button"
                    key={item.id}
                    aria-current={view === item.id ? "page" : undefined}
                    onClick={() => navigate(item.id)}
                  >
                    <Icon name={item.icon} size={17} />
                    <span>{item.label}</span>
                    {item.id === "discover" && <kbd>⌘ K</kbd>}
                  </button>
                ))}
            </div>
          ))}
        </nav>
        <div className="ws-sidebar-bottom">
          <SidebarBalance
            token={token}
            credentialId={credential?.id}
            onOpen={() => navigate("settings", "balance")}
          />
          <Link href="/docs" className="ws-sidebar-link">
            <Icon name="book" size={15} />
            Docs &amp; guidance
            <Icon name="external" size={12} />
          </Link>
          <button
            className="ws-account"
            type="button"
            onClick={() => navigate("settings")}
          >
            <span className="ws-avatar dark">
              {accountProfile.data?.avatar ? (
                <Image
                  unoptimized
                  src={accountProfile.data.avatar}
                  alt=""
                  width={32}
                  height={32}
                  referrerPolicy="no-referrer"
                />
              ) : (
                <span aria-hidden="true">H</span>
              )}
            </span>
            <span>
              <strong>
                {accountProfile.data?.username ??
                  credential?.name ??
                  "Hive account"}
              </strong>
              <small>Account settings</small>
            </span>
            <Icon name="chevron" size={14} />
          </button>
        </div>
      </aside>
      <div className="ws-main" inert={mobileOpen}>
        <header className="ws-topbar">
          <div className="ws-breadcrumb">
            <button
              type="button"
              className="ws-icon-button ws-mobile-menu"
              aria-label="Open navigation"
              aria-expanded={mobileOpen}
              onClick={() => setMobileOpen(true)}
            >
              <Icon name="menu" size={18} />
            </button>
            <span>Workspace</span>
            <Icon name="chevron" size={12} />
            <strong>
              {navigation.find((item) => item.id === view)?.label}
            </strong>
          </div>
          <div className="ws-topbar-actions">
            <span className="ws-account-tier">{plan.data?.tier ?? "…"}</span>
            <span
              className="ws-account-credits"
              title="Daily hosted build credits; separate from wallet funds"
            >
              {plan.data?.build_credits
                ? `${plan.data.build_credits.available} build credits`
                : "Build credits unavailable"}
            </span>
          </div>
        </header>
        <main id="workspace-content" className="ws-content">
          {view !== "discover" && (
            <div
              className={`ws-page-heading ${view === "settings" ? "ws-account-page-heading" : ""}`}
            >
              <div>
                {view !== "settings" && (
                  <span className="ws-overline">YOUR WORKSPACE</span>
                )}
                <h1>{head.title}</h1>
                {view !== "settings" && <p>{head.description}</p>}
              </div>
            </div>
          )}
          {error && (
            <div className="ws-error-banner" role="alert">
              <span>{error}</span>
              <button
                type="button"
                className="ws-text-button"
                onClick={refreshData}
              >
                Retry
              </button>
              <button
                type="button"
                className="ws-text-button"
                onClick={() => setConnectOpen(true)}
              >
                Reconnect
              </button>
            </div>
          )}
          {/* Each view renders a different workspace surface. */}
          {/* eslint-disable no-nested-ternary */}
          {catalogView ? (
            <div
              className={`ws-discovery ${detailOpen ? "detail-is-open" : ""}`}
            >
              <section className="ws-catalog" aria-label="Capability registry">
                <h1 className="sr-only">Discover</h1>
                <div className="ws-search-area">
                  <search>
                    <form
                      onSubmit={(e) => {
                        e.preventDefault();
                        if (results[0]) {
                          openDetail(results[0].id);
                        }
                      }}
                    >
                      <Icon name="search" size={19} />
                      <input
                        ref={search}
                        value={query}
                        onChange={(e) => {
                          if (
                            new TextEncoder().encode(e.target.value).length <=
                            200
                          ) {
                            setQuery(e.target.value);
                          }
                        }}
                        placeholder="Search by name or description…"
                        autoComplete="off"
                        spellCheck={false}
                        aria-label="Search capabilities"
                      />
                      {query ? (
                        <button
                          type="button"
                          className="ws-icon-button"
                          aria-label="Clear search"
                          onClick={() => {
                            setQuery("");
                            search.current?.focus();
                          }}
                        >
                          <Icon name="close" size={14} />
                        </button>
                      ) : (
                        <kbd>⌘ K</kbd>
                      )}
                      <button
                        type="submit"
                        className="ws-search-submit"
                        aria-label="Open first result"
                      >
                        <Icon name="arrow" size={16} />
                      </button>
                    </form>
                  </search>
                  <div className="ws-filterbar">
                    <div
                      className="ws-filters"
                      aria-label="Filter loaded capabilities by runtime"
                    >
                      {["all", ...Object.keys(runtimeLabels)].map((runtime) => (
                        <button
                          type="button"
                          aria-pressed={filter === runtime}
                          key={runtime}
                          onClick={() => setFilter(runtime)}
                        >
                          {runtime === "all"
                            ? "All capabilities"
                            : runtimeLabels[runtime as Runtime]}
                          {(!token ||
                            (runtime === filter &&
                              !registry.isPlaceholderData)) && (
                            <span>
                              {
                                visibleCatalog.filter(
                                  (tool) =>
                                    tool.active &&
                                    (runtime === "all" ||
                                      tool.runtime === runtime)
                                ).length
                              }
                            </span>
                          )}
                        </button>
                      ))}
                    </div>
                    <label className="ws-sort">
                      <span>Sort</span>
                      <select
                        value={sort}
                        aria-label="Sort loaded capabilities"
                        onChange={(e) => setSort(e.target.value)}
                      >
                        <option value="newest">Newest first</option>
                        <option value="price">Lowest price</option>
                        <option value="name">Name A–Z</option>
                      </select>
                    </label>
                  </div>
                </div>
                <div className="ws-results-caption">
                  <output>
                    {loading
                      ? "Refreshing registry…"
                      : `${results.length} ${results.length === 1 ? "capability" : "capabilities"}`}
                  </output>
                  <span>
                    {token
                      ? "Filters apply to loaded results"
                      : "Illustrative prices · no live execution"}
                  </span>
                </div>
                {metadata.error && (
                  <p role="alert">{errorMessage(metadata.error)}</p>
                )}
                <div className="ws-list" aria-busy={loading}>
                  {results.map((tool, index) => (
                    <button
                      className={`ws-capability-row ${selected?.id === tool.id ? "is-selected" : ""}`}
                      key={tool.id}
                      type="button"
                      aria-pressed={selected?.id === tool.id}
                      onClick={() => openDetail(tool.id)}
                    >
                      <span className="ws-row-number">
                        {String(index + 1).padStart(2, "0")}
                      </span>
                      <span className="ws-row-info">
                        <strong>
                          {tool.name}
                          <span className="ws-row-version">
                            v{tool.version}
                          </span>
                        </strong>
                        <span className="ws-row-description">
                          {tool.description}
                        </span>
                        <span className="ws-row-meta">
                          <span>{runtimeLabels[tool.runtime]}</span>
                          <PublisherSummary metadata={tool.metadata} />
                        </span>
                      </span>
                      <span className="ws-row-price">
                        <strong>{atomicMoney(tool.price)}</strong>
                        <small>USDC / call</small>
                      </span>
                      <Icon
                        className="ws-row-chevron"
                        name="chevron"
                        size={15}
                      />
                    </button>
                  ))}
                  {!results.length && (
                    <EmptyState title="No matching capabilities">
                      Try another name or reset the runtime filter.
                      <button
                        className="ws-text-button"
                        type="button"
                        onClick={() => {
                          setQuery("");
                          setFilter("all");
                        }}
                      >
                        Clear search &amp; filters{" "}
                        <Icon name="arrow" size={14} />
                      </button>
                    </EmptyState>
                  )}
                </div>
                {token &&
                  registry.hasNextPage &&
                  !registry.isPlaceholderData && (
                    <button
                      className="ws-button"
                      type="button"
                      disabled={registry.isFetchingNextPage}
                      onClick={() => {
                        void registry.fetchNextPage();
                      }}
                    >
                      Load more capabilities
                    </button>
                  )}
              </section>
              <aside
                ref={inspector}
                className="ws-inspector"
                aria-label="Selected capability"
              >
                {selected ? (
                  <Inspector
                    key={selected.id}
                    tool={selected}
                    catalog={visibleCatalog}
                    token={token}
                    onConnect={() => setConnectOpen(true)}
                    onRefresh={refreshData}
                    onSelect={(id) => {
                      setFilter("all");
                      setQuery("");
                      openDetail(id);
                    }}
                    onClose={() => setDetailOpen(false)}
                  />
                ) : (
                  <EmptyState title="Details live here">
                    Select a capability to inspect its contract.
                  </EmptyState>
                )}
              </aside>
            </div>
          ) : view === "executions" ? (
            <ActivityPanel
              key={credential?.id ?? "preview"}
              token={token}
              activity={activity}
              hasMore={history.hasNextPage}
              loadingMore={history.isFetchingNextPage}
              onLoadMore={() => {
                void history.fetchNextPage();
              }}
              catalog={catalog}
              onRefresh={refreshData}
            />
          ) : view === "billing" ? (
            <SalesPanel
              sales={sales}
              enabled={Boolean(credential?.scopes.includes("sales:read"))}
              onConnect={() => setConnectOpen(true)}
            />
          ) : (
            <AccountPanel
              credential={credential}
              capabilities={catalog}
              token={token}
              tab={accountTab}
              onTabChange={(next) => navigate("settings", next)}
              hasMore={Boolean(registry.hasNextPage)}
              onLoadMore={() => {
                void registry.fetchNextPage();
              }}
              loading={loading}
              onRefresh={refreshData}
              onConnect={() => setConnectOpen(true)}
              onDisconnect={disconnect}
            />
          )}
          {/* eslint-enable no-nested-ternary */}
        </main>
        {view !== "discover" && (
          <footer className="ws-footer">
            <span>Built by agents. Used by agents.</span>
            <span>
              hive<span className="ws-footer-dot">.</span>
            </span>
          </footer>
        )}
      </div>
      {connectOpen && (
        <SessionDialog
          onClose={() => setConnectOpen(false)}
          onConnect={(issued, connected) => {
            setSelectedId("");
            setToken(issued);
            setCredential(connected);
          }}
        />
      )}
    </div>
  );
};
export default Workspace;
