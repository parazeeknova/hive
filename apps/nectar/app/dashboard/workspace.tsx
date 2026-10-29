"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";

import { Brand } from "../../components/brand";
import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";
import {
  resolveAccountCapability,
  useAccountPreview,
} from "../../lib/account-store";
import {
  atomicMoney,
  errorMessage,
  exampleCapabilities,
  hiveway,
  runtimeLabels,
  shortId,
} from "../../lib/hiveway";
import type {
  Activity,
  Credential,
  RegistryCapability,
  Runtime,
  Sale,
} from "../../lib/hiveway";
import AccountPanel from "./account-panel";
import type { AccountTab } from "./account-panel";
import { SidebarBalance } from "./account-payments";
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
  const [action, setAction] = useState<"sample" | "invoke" | null>(null);
  const steps = Array.isArray(tool.definition.steps)
    ? tool.definition.steps.filter((id): id is string => typeof id === "string")
    : [];
  const authorName = shortId(tool.creator_wallet);
  const initials = tool.creator_wallet.startsWith("0x")
    ? tool.creator_wallet.slice(2, 4).toUpperCase()
    : "EC";
  return (
    <>
      <div className="ws-author-row">
        <span className="ws-author-avatar" aria-hidden="true">
          {initials}
        </span>
        <span className="ws-author-name" title={tool.creator_wallet}>
          {authorName}
        </span>
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
      <div className="ws-capability-tags">
        <span>{runtimeLabels[tool.runtime]}</span>
        <span>GitHub</span>
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
            <p>Required input</p>
            <Json
              value={{
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
              }}
            />
            <p>
              {tool.runtime === "workflow"
                ? "Output contains repository identity and a steps array with each capability, version, runtime, and result."
                : "Report fields can include repository_url, revision, manifests, technologies, dependencies, and analysis_limit_reached. The settled result is the source of truth."}
            </p>
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
              Public GitHub repositories only. Supply a full 40-character commit
              SHA; branches, tags, and private repositories are not supported.
            </p>
            <p>
              {tool.runtime === "workflow"
                ? "Every step uses the same repository input. Conditional steps and nested workflows are not supported."
                : "Analysis describes declared repository manifests and may reach runtime analysis limits."}
            </p>
          </div>
        </details>
        <details className="ws-detail-section">
          <summary>
            <span>Provider</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body">
            <p>Creator wallet</p>
            <code className="ws-provider-wallet">{tool.creator_wallet}</code>
            <p>
              Version {tool.version}. Author usernames and profile photos are
              not available from the current API.
            </p>
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
    return tab === "balance" || tab === "capabilities" ? tab : "profile";
  });
  const [query, setQuery] = useState(params.get("q") ?? "");
  const [filter, setFilter] = useState("all");
  const [sort, setSort] = useState("newest");
  const [selectedId, setSelectedId] = useState(
    params.get("tool") ?? "example-stack"
  );
  const [detailOpen, setDetailOpen] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [connectOpen, setConnectOpen] = useState(false);
  const [token, setToken] = useState("");
  const [credential, setCredential] = useState<Credential | null>(null);
  const accountScope = credential?.wallet.toLowerCase() ?? "preview";
  const { account, updateAccount } = useAccountPreview(accountScope);
  const [catalog, setCatalog] =
    useState<RegistryCapability[]>(exampleCapabilities);
  const [activity, setActivity] = useState<Activity | null>(null);
  const [sales, setSales] = useState<Sale[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [refresh, setRefresh] = useState(0);
  const search = useRef<HTMLInputElement>(null);
  const inspector = useRef<HTMLElement>(null);
  const refreshData = useCallback(() => setRefresh((value) => value + 1), []);
  const disconnect = () => {
    setToken("");
    setCredential(null);
    setActivity(null);
    setSales([]);
    setCatalog(exampleCapabilities);
    setLoading(false);
    setError("");
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
    if (!token) {
      return;
    }
    let disposed = false;
    const controller = new AbortController();
    const timer = setTimeout(async () => {
      setError("");
      try {
        const me = await hiveway<Credential>("/v1/credentials/me", token, {
          signal: controller.signal,
        });
        if (disposed) {
          return;
        }
        setCredential(me);
        const results = await Promise.allSettled([
          me.scopes.includes("receipts:read")
            ? hiveway<Activity>("/v1/activity", token, {
                signal: controller.signal,
              })
            : Promise.resolve(null),
          me.scopes.includes("sales:read")
            ? hiveway<Sale[]>("/v1/sales", token, { signal: controller.signal })
            : Promise.resolve([]),
        ]);
        if (disposed) {
          return;
        }
        const [history, earnings] = results;
        if (history.status === "fulfilled") {
          setActivity(history.value);
        }
        if (earnings.status === "fulfilled") {
          setSales(earnings.value);
        }
        const rejected = results.find((result) => result.status === "rejected");
        if (rejected?.status === "rejected") {
          setError(errorMessage(rejected.reason));
        }
      } catch (error_) {
        if (!disposed) {
          setError(errorMessage(error_));
        }
      }
    }, 0);
    return () => {
      disposed = true;
      controller.abort();
      clearTimeout(timer);
    };
    // Refresh deliberately re-runs the loader without changing its inputs.
    // eslint-disable-next-line react/exhaustive-effect-dependencies
  }, [token, refresh]);
  const canReadCatalog =
    credential?.scopes.includes("capabilities:read") ?? false;
  useEffect(() => {
    if (!token || !canReadCatalog) {
      return;
    }
    const controller = new AbortController();
    let disposed = false;
    const timer = setTimeout(async () => {
      setLoading(true);
      try {
        const loaded = await hiveway<RegistryCapability[]>(
          `/v1/capabilities?q=${encodeURIComponent(view === "discover" ? query : "")}`,
          token,
          { signal: controller.signal }
        );
        if (!disposed) {
          setCatalog(loaded);
        }
      } catch (error_) {
        if (!disposed) {
          setError(errorMessage(error_));
        }
      }
      if (!disposed) {
        setLoading(false);
      }
    }, 250);
    return () => {
      disposed = true;
      controller.abort();
      clearTimeout(timer);
    };
    // Refresh also reloads the current registry query.
    // eslint-disable-next-line react/exhaustive-effect-dependencies
  }, [token, canReadCatalog, view, query, refresh]);
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

  const visibleCatalog = token
    ? catalog
    : catalog.map((item) =>
        resolveAccountCapability(item, account.capabilities)
      );
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
            balance={account.balance}
            preview={accountScope === "preview"}
            onOpen={() => navigate("settings", "balance")}
          />
          <Link href="/#docs" className="ws-sidebar-link">
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
              <Icon name="wallet" size={16} />
            </span>
            <span>
              <strong>{account.profile.name}</strong>
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
            <button
              type="button"
              className="ws-button ws-primary ws-small"
              onClick={() => setConnectOpen(true)}
            >
              <Icon name="wallet" size={14} />
              {credential ? "Change session" : "Connect wallet"}
            </button>
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
                      {[
                        "all",
                        "repository_stack",
                        "dependency_inventory",
                        "workflow",
                      ].map((runtime) => (
                        <button
                          type="button"
                          aria-pressed={filter === runtime}
                          key={runtime}
                          onClick={() => setFilter(runtime)}
                        >
                          {runtime === "all"
                            ? "All capabilities"
                            : runtimeLabels[runtime as Runtime]}
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
                          <span>
                            {token
                              ? shortId(tool.creator_wallet)
                              : "Example capability"}
                          </span>
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
              key={accountScope}
              account={account}
              tab={accountTab}
              onTabChange={(next) => navigate("settings", next)}
              scope={accountScope}
              onUpdate={updateAccount}
              credential={credential}
              capabilities={
                credential
                  ? catalog.filter(
                      (item) =>
                        item.creator_wallet.toLowerCase() ===
                        credential.wallet.toLowerCase()
                    )
                  : exampleCapabilities
              }
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
          onConnect={(issued, me) => {
            setCatalog([]);
            setSelectedId("");
            setToken(issued);
            setCredential(me);
            setActivity(null);
            setSales([]);
          }}
        />
      )}
    </div>
  );
};
export default Workspace;
