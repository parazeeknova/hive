"use client";
import {
  keepPreviousData,
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
      <div className="ws-author-row mb-[19px] flex items-center gap-[9px] [&_.ws-detail-close]:ml-auto">
        <span className="ws-author-name min-w-0 overflow-hidden text-[12px] font-semibold text-ellipsis whitespace-nowrap text-[#4c5545]">
          Capability details
        </span>
        <button
          className="ws-icon-button ws-detail-close hidden h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] max-[1100px]:flex [&:hover]:bg-[#edeee7]"
          type="button"
          onClick={onClose}
          aria-label="Close capability details"
        >
          <Icon name="close" size={16} />
        </button>
      </div>
      <div className="ws-tool-title mb-2.5 flex items-baseline gap-2 [&_h2]:[flex:1] [&_h2]:text-[17px] [&_h2]:leading-[1.5] [&_h2]:[font-weight:650] [&_h2]:tracking-[-0.045em]">
        <h2 className="font-display">{tool.name}</h2>
      </div>
      <p className="ws-capability-description text-[12px] leading-[1.85] [overflow-wrap:anywhere] text-[#62695b]">
        {tool.description}
      </p>
      <PublisherSummary metadata={tool.metadata} />
      <div className="ws-capability-tags mt-3.5 flex flex-wrap gap-1.5 [&>span]:rounded-[5px] [&>span]:bg-[#f5f6f2] [&>span]:px-[7px] [&>span]:py-1 [&>span]:text-[11px] [&>span]:leading-[1.4] [&>span]:text-[#5d6653] [&>span]:[border:1px_solid_#e0e4da]">
        <span>{runtimeLabels[tool.runtime]}</span>
        <span>{tool.runtime === "wasm" ? "Sandboxed" : "GitHub"}</span>
      </div>
      <div className="ws-inspector-price mx-0 my-[17px] flex flex-wrap items-center justify-between gap-2 [&_small]:text-[10px] [&_small]:font-normal [&_small]:tracking-[0] [&_small]:text-[#686f60] [&>strong]:text-[20px] [&>strong]:font-semibold [&>strong]:tracking-[-0.05em] [&>strong]:text-[#566447]">
        <strong>
          {atomicMoney(tool.price)} <small>USDC / call</small>
        </strong>
      </div>
      <div className="ws-detail-sections [border-top:1px_solid_var(--ws-line)]">
        <details className="ws-detail-section [border-bottom:1px_solid_var(--ws-line)] [&>summary]:flex [&>summary]:min-h-[54px] [&>summary]:items-center [&>summary]:justify-between [&>summary]:gap-3 [&>summary]:px-0 [&>summary]:py-4 [&>summary]:text-[12px] [&>summary]:[font-weight:550] [&>summary]:text-[#424b38] [&>summary]:[list-style:none] [&>summary_small]:ml-1 [&>summary_small]:text-[11px] [&>summary_small]:font-normal [&>summary_small]:text-[#717c64] [&>summary::-webkit-details-marker]:hidden [&>summary:hover]:text-[#24331a] [&>summary>svg]:shrink-0 [&>summary>svg]:text-[#7a856e] [&>summary>svg]:[transition:transform_0.15s] [&[open]>summary>svg]:[transform:rotate(90deg)]">
          <summary className="cursor-pointer">
            <span>Overview</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body px-0 pt-0 pb-[17px] text-[12px] leading-[1.85] text-[#626d56] [&_.ws-json]:text-[11px] [&_.ws-json]:text-[#56644a] [&_code]:text-[11px] [&_code]:[overflow-wrap:anywhere] [&_p+p]:mt-2.5">
            <p>{tool.description}</p>
            <p>
              Runtime: <code>{tool.runtime}</code>. Version {tool.version}.
            </p>
          </div>
        </details>
        <details className="ws-detail-section [border-bottom:1px_solid_var(--ws-line)] [&>summary]:flex [&>summary]:min-h-[54px] [&>summary]:items-center [&>summary]:justify-between [&>summary]:gap-3 [&>summary]:px-0 [&>summary]:py-4 [&>summary]:text-[12px] [&>summary]:[font-weight:550] [&>summary]:text-[#424b38] [&>summary]:[list-style:none] [&>summary_small]:ml-1 [&>summary_small]:text-[11px] [&>summary_small]:font-normal [&>summary_small]:text-[#717c64] [&>summary::-webkit-details-marker]:hidden [&>summary:hover]:text-[#24331a] [&>summary>svg]:shrink-0 [&>summary>svg]:text-[#7a856e] [&>summary>svg]:[transition:transform_0.15s] [&[open]>summary>svg]:[transform:rotate(90deg)]">
          <summary className="cursor-pointer">
            <span>Input / Output Schema</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body px-0 pt-0 pb-[17px] text-[12px] leading-[1.85] text-[#626d56] [&_.ws-json]:text-[11px] [&_.ws-json]:text-[#56644a] [&_code]:text-[11px] [&_code]:[overflow-wrap:anywhere] [&_p+p]:mt-2.5">
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
        <details className="ws-detail-section [border-bottom:1px_solid_var(--ws-line)] [&>summary]:flex [&>summary]:min-h-[54px] [&>summary]:items-center [&>summary]:justify-between [&>summary]:gap-3 [&>summary]:px-0 [&>summary]:py-4 [&>summary]:text-[12px] [&>summary]:[font-weight:550] [&>summary]:text-[#424b38] [&>summary]:[list-style:none] [&>summary_small]:ml-1 [&>summary_small]:text-[11px] [&>summary_small]:font-normal [&>summary_small]:text-[#717c64] [&>summary::-webkit-details-marker]:hidden [&>summary:hover]:text-[#24331a] [&>summary>svg]:shrink-0 [&>summary>svg]:text-[#7a856e] [&>summary>svg]:[transition:transform_0.15s] [&[open]>summary>svg]:[transform:rotate(90deg)]">
          <summary className="cursor-pointer">
            <span>Sample Response</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body px-0 pt-0 pb-[17px] text-[12px] leading-[1.85] text-[#626d56] [&_.ws-json]:text-[11px] [&_.ws-json]:text-[#56644a] [&_code]:text-[11px] [&_code]:[overflow-wrap:anywhere] [&_p+p]:mt-2.5">
            <p>
              A sample response is not provided by the registry yet. Your actual
              settled output is available in Activity after execution.
            </p>
          </div>
        </details>
        <details
          className="ws-detail-section [border-bottom:1px_solid_var(--ws-line)] [&>summary]:flex [&>summary]:min-h-[54px] [&>summary]:items-center [&>summary]:justify-between [&>summary]:gap-3 [&>summary]:px-0 [&>summary]:py-4 [&>summary]:text-[12px] [&>summary]:[font-weight:550] [&>summary]:text-[#424b38] [&>summary]:[list-style:none] [&>summary_small]:ml-1 [&>summary_small]:text-[11px] [&>summary_small]:font-normal [&>summary_small]:text-[#717c64] [&>summary::-webkit-details-marker]:hidden [&>summary:hover]:text-[#24331a] [&>summary>svg]:shrink-0 [&>summary>svg]:text-[#7a856e] [&>summary>svg]:[transition:transform_0.15s] [&[open]>summary>svg]:[transform:rotate(90deg)]"
          open={steps.length > 0}
        >
          <summary className="cursor-pointer">
            <span>
              Dependencies <small>({steps.length})</small>
            </span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body px-0 pt-0 pb-[17px] text-[12px] leading-[1.85] text-[#626d56] [&_.ws-json]:text-[11px] [&_.ws-json]:text-[#56644a] [&_code]:text-[11px] [&_code]:[overflow-wrap:anywhere] [&_p+p]:mt-2.5">
            {steps.length > 0 ? (
              <div className="ws-dependency-tree my-0 mr-0 ml-1.5 [border-left:1px_solid_#cbd4bf] [&_button]:relative [&_button]:flex [&_button]:w-full [&_button]:items-baseline [&_button]:justify-between [&_button]:gap-3 [&_button]:py-[9px] [&_button]:pr-0 [&_button]:pl-[18px] [&_button]:text-left [&_button]:text-[12px] [&_button]:leading-[1.6] [&_button]:text-[#546347] [&_button]:[background:none] [&_button]:[border:0] [&_button_small]:shrink-0 [&_button_small]:text-[10px] [&_button_small]:text-[#68765b] [&_button_span]:min-w-0 [&_button_span]:[overflow-wrap:anywhere] [&_button::before]:absolute [&_button::before]:top-[19px] [&_button::before]:left-0 [&_button::before]:h-px [&_button::before]:w-2.5 [&_button::before]:bg-[#cbd4bf] [&_button::before]:[content:''] [&_button:hover:not(:disabled)]:text-[#2b411d] [&_button:hover:not(:disabled)]:[text-decoration:underline]">
                {steps.map((id) => {
                  const dependency = catalog.find((item) => item.id === id);
                  return (
                    <button
                      type="button"
                      key={id}
                      disabled={!dependency}
                      onClick={() => onSelect(id)}
                      className="cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45]"
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
        <details className="ws-detail-section [border-bottom:1px_solid_var(--ws-line)] [&>summary]:flex [&>summary]:min-h-[54px] [&>summary]:items-center [&>summary]:justify-between [&>summary]:gap-3 [&>summary]:px-0 [&>summary]:py-4 [&>summary]:text-[12px] [&>summary]:[font-weight:550] [&>summary]:text-[#424b38] [&>summary]:[list-style:none] [&>summary_small]:ml-1 [&>summary_small]:text-[11px] [&>summary_small]:font-normal [&>summary_small]:text-[#717c64] [&>summary::-webkit-details-marker]:hidden [&>summary:hover]:text-[#24331a] [&>summary>svg]:shrink-0 [&>summary>svg]:text-[#7a856e] [&>summary>svg]:[transition:transform_0.15s] [&[open]>summary>svg]:[transform:rotate(90deg)]">
          <summary className="cursor-pointer">
            <span>Pricing</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body px-0 pt-0 pb-[17px] text-[12px] leading-[1.85] text-[#626d56] [&_.ws-json]:text-[11px] [&_.ws-json]:text-[#56644a] [&_code]:text-[11px] [&_code]:[overflow-wrap:anywhere] [&_p+p]:mt-2.5">
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
        <details className="ws-detail-section [border-bottom:1px_solid_var(--ws-line)] [&>summary]:flex [&>summary]:min-h-[54px] [&>summary]:items-center [&>summary]:justify-between [&>summary]:gap-3 [&>summary]:px-0 [&>summary]:py-4 [&>summary]:text-[12px] [&>summary]:[font-weight:550] [&>summary]:text-[#424b38] [&>summary]:[list-style:none] [&>summary_small]:ml-1 [&>summary_small]:text-[11px] [&>summary_small]:font-normal [&>summary_small]:text-[#717c64] [&>summary::-webkit-details-marker]:hidden [&>summary:hover]:text-[#24331a] [&>summary>svg]:shrink-0 [&>summary>svg]:text-[#7a856e] [&>summary>svg]:[transition:transform_0.15s] [&[open]>summary>svg]:[transform:rotate(90deg)]">
          <summary className="cursor-pointer">
            <span>Limitations</span>
            <Icon name="chevron" size={15} />
          </summary>
          <div className="ws-section-body px-0 pt-0 pb-[17px] text-[12px] leading-[1.85] text-[#626d56] [&_.ws-json]:text-[11px] [&_.ws-json]:text-[#56644a] [&_code]:text-[11px] [&_code]:[overflow-wrap:anywhere] [&_p+p]:mt-2.5">
            <p>
              {tool.runtime === "wasm"
                ? "Pure JSON-in/JSON-out Wasm. No filesystem, network, clock, or environment access."
                : "Public GitHub repositories only. Supply a full 40-character commit SHA; branches, tags, and private repositories are not supported."}
            </p>
            <p>{limitationDescription}</p>
          </div>
        </details>
      </div>
      <div className="ws-capability-actions mt-6 flex flex-col gap-2.5 [&_.ws-button]:min-h-[44px] [&_.ws-button]:rounded-[8px] [&_.ws-button]:text-[12px] [&_.ws-button.ws-primary]:[background:#292e25] [&_.ws-button.ws-primary:hover]:[background:#3d4734]">
        <button
          type="button"
          className="ws-button ws-primary inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#40473a] [&:hover]:bg-[#f4f5ee]"
          onClick={() => setAction("sample")}
        >
          <Icon name="play" size={16} />
          Test with sample input
        </button>
        <button
          type="button"
          className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
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
    placeholderData: keepPreviousData,
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
    <div
      className={`ws-app grid min-h-dvh [grid-template-columns:200px_minmax(0,_1fr)] text-[13px] text-[color:#292a26] [--ws-green:#52664b] [--ws-line:#e7e6e2] [--ws-muted:#797973] [background:#fafaf8] max-[1280px]:[grid-template-columns:180px_minmax(0,_1fr)] max-[760px]:block min-[1700px]:[grid-template-columns:218px_minmax(0,_1fr)] [&_a]:[transition:background_0.15s,_border-color_0.15s,_color_0.15s] [&_a:focus-visible]:[outline:2px_solid_#768a60] [&_a:focus-visible]:[outline-offset:3px] [&_button]:[transition:background_0.15s,_border-color_0.15s,_color_0.15s] [&_button:focus-visible]:[outline:2px_solid_#768a60] [&_button:focus-visible]:[outline-offset:3px] [&_input]:[transition:background_0.15s,_border-color_0.15s,_color_0.15s] [&_input:focus-visible]:[outline:2px_solid_#768a60] [&_input:focus-visible]:[outline-offset:3px] [&_select]:[transition:background_0.15s,_border-color_0.15s,_color_0.15s] [&_select:focus-visible]:[outline:2px_solid_#768a60] [&_select:focus-visible]:[outline-offset:3px] [&_textarea]:[transition:background_0.15s,_border-color_0.15s,_color_0.15s] [&_textarea:focus-visible]:[outline:2px_solid_#768a60] [&_textarea:focus-visible]:[outline-offset:3px] [@media_(prefers-reduced-motion:reduce)]:[&_*]:[scroll-behavior:auto]! [@media_(prefers-reduced-motion:reduce)]:[&_*]:[transition:none]! [@media_(prefers-reduced-motion:reduce)]:[&_*::after]:[scroll-behavior:auto]! [@media_(prefers-reduced-motion:reduce)]:[&_*::after]:[transition:none]! [@media_(prefers-reduced-motion:reduce)]:[&_*::before]:[scroll-behavior:auto]! [@media_(prefers-reduced-motion:reduce)]:[&_*::before]:[transition:none]! ${view === "discover" ? "ws-discover-view [&_.ws-capability-row]:min-h-[105px] [&_.ws-capability-row]:gap-4 [&_.ws-capability-row]:pt-[18px] [&_.ws-capability-row]:pr-[18px] [&_.ws-capability-row]:pb-[18px] [&_.ws-capability-row]:pl-[18px] max-[760px]:[&_.ws-capability-row]:items-start max-[760px]:[&_.ws-capability-row]:gap-3 max-[760px]:[&_.ws-capability-row]:pt-[17px] max-[760px]:[&_.ws-capability-row]:pr-[13px] max-[760px]:[&_.ws-capability-row]:pb-[17px] max-[760px]:[&_.ws-capability-row]:pl-[13px] [&_.ws-capability-row.is-selected]:[background:#f1f5e9] [&_.ws-capability-row.is-selected::before]:top-[17px] [&_.ws-capability-row.is-selected::before]:bottom-[17px] [&_.ws-capability-row.is-selected::before]:[background:#7a8f60] [&_.ws-catalog]:pt-[27px] [&_.ws-catalog]:pr-[26px] [&_.ws-catalog]:pb-9 [&_.ws-catalog]:pl-[26px] max-[1280px]:[&_.ws-catalog]:pt-[23px] max-[1280px]:[&_.ws-catalog]:pr-5 max-[1280px]:[&_.ws-catalog]:pb-[23px] max-[1280px]:[&_.ws-catalog]:pl-5 max-[1100px]:[&_.ws-catalog]:pt-0 max-[1100px]:[&_.ws-catalog]:pr-0 max-[1100px]:[&_.ws-catalog]:pb-0 max-[1100px]:[&_.ws-catalog]:pl-0 min-[1700px]:[&_.ws-catalog]:pt-8 min-[1700px]:[&_.ws-catalog]:pr-8 min-[1700px]:[&_.ws-catalog]:pb-8 min-[1700px]:[&_.ws-catalog]:pl-8 [&_.ws-content]:flex [&_.ws-content]:flex-col [&_.ws-content]:pt-0 [&_.ws-content]:pr-0 [&_.ws-content]:pb-0 [&_.ws-content]:pl-0 max-[1100px]:[&_.ws-content]:pt-[23px] max-[1100px]:[&_.ws-content]:pr-5 max-[1100px]:[&_.ws-content]:pb-[23px] max-[1100px]:[&_.ws-content]:pl-5 max-[760px]:[&_.ws-content]:pt-6 max-[760px]:[&_.ws-content]:pr-4 max-[760px]:[&_.ws-content]:pb-6 max-[760px]:[&_.ws-content]:pl-4 [&_.ws-create-capability]:w-auto [&_.ws-create-capability]:shrink-0 [&_.ws-create-capability]:gap-2 [&_.ws-create-capability]:text-[11px] max-[600px]:[&_.ws-create-capability]:min-h-10 max-[600px]:[&_.ws-create-capability]:w-10 max-[600px]:[&_.ws-create-capability]:pt-2.5 max-[600px]:[&_.ws-create-capability]:pr-2.5 max-[600px]:[&_.ws-create-capability]:pb-2.5 max-[600px]:[&_.ws-create-capability]:pl-2.5 [&_.ws-discover-heading]:gap-3 max-[760px]:[&_.ws-discover-heading]:items-center max-[760px]:[&_.ws-discover-heading_h1]:text-[20px] [&_.ws-discovery]:min-h-[calc(100dvh_-_64px)] [&_.ws-discovery]:[flex:1] [&_.ws-discovery]:gap-0 max-[1100px]:[&_.ws-discovery]:min-h-0 max-[1100px]:[&_.ws-discovery]:gap-[18px] [&_.ws-error-banner]:mt-3.5 [&_.ws-error-banner]:mr-[26px] [&_.ws-error-banner]:mb-3.5 [&_.ws-error-banner]:ml-[26px] max-[1100px]:[&_.ws-error-banner]:mt-0 max-[1100px]:[&_.ws-error-banner]:mr-0 max-[1100px]:[&_.ws-error-banner]:mb-[18px] max-[1100px]:[&_.ws-error-banner]:ml-0 [&_.ws-filterbar]:flex-wrap [&_.ws-filterbar]:items-center [&_.ws-filterbar]:gap-3 [&_.ws-filterbar]:pt-0 [&_.ws-filterbar]:pr-4 [&_.ws-filterbar]:pb-[15px] [&_.ws-filterbar]:pl-4 max-[760px]:[&_.ws-filterbar]:items-stretch max-[760px]:[&_.ws-filterbar]:gap-2.5 max-[760px]:[&_.ws-filterbar]:pt-0 max-[760px]:[&_.ws-filterbar]:pr-3 max-[760px]:[&_.ws-filterbar]:pb-3 max-[760px]:[&_.ws-filterbar]:pl-3 [&_.ws-filters]:gap-[5px] [&_.ws-filters_button]:min-h-9 [&_.ws-filters_button]:gap-[7px] [&_.ws-filters_button]:rounded-[7px] [&_.ws-filters_button]:pt-1.5 [&_.ws-filters_button]:pr-2.5 [&_.ws-filters_button]:pb-1.5 [&_.ws-filters_button]:pl-2.5 [&_.ws-filters_button]:text-[12px] [&_.ws-filters_button]:[font-weight:550] [&_.ws-filters_button]:text-[color:#5e6755] max-[760px]:[&_.ws-filters_button]:min-h-10 max-[760px]:[&_.ws-filters_button]:pt-[7px] max-[760px]:[&_.ws-filters_button]:pr-[9px] max-[760px]:[&_.ws-filters_button]:pb-[7px] max-[760px]:[&_.ws-filters_button]:pl-[9px] max-[760px]:[&_.ws-filters_button]:text-[11px] [&_.ws-filters_button>span]:inline-flex [&_.ws-filters_button>span]:min-w-[19px] [&_.ws-filters_button>span]:items-center [&_.ws-filters_button>span]:justify-center [&_.ws-filters_button>span]:rounded-[4px] [&_.ws-filters_button>span]:pt-0.5 [&_.ws-filters_button>span]:pr-1 [&_.ws-filters_button>span]:pb-0.5 [&_.ws-filters_button>span]:pl-1 [&_.ws-filters_button>span]:text-[10px] [&_.ws-filters_button>span]:leading-[1.3] [&_.ws-filters_button>span]:text-[color:#5c6850] [&_.ws-filters_button>span]:[font-variant-numeric:tabular-nums] [&_.ws-filters_button>span]:opacity-[1] [&_.ws-filters_button>span]:[background:#f0f2eb] [&_.ws-filters_button[aria-pressed='true']]:[border-color:#30382a] [&_.ws-filters_button[aria-pressed='true']]:text-[color:#fff] [&_.ws-filters_button[aria-pressed='true']]:[background:#30382a] [&_.ws-filters_button[aria-pressed='true']>span]:text-[color:#fff] [&_.ws-filters_button[aria-pressed='true']>span]:[background:#ffffff1c] [&_.ws-inspector]:top-16 [&_.ws-inspector]:h-[calc(100dvh_-_64px)] [&_.ws-inspector]:overflow-y-auto [&_.ws-inspector]:[overscroll-behavior-y:contain] [&_.ws-inspector]:rounded-none [&_.ws-inspector]:[box-shadow:none] [&_.ws-inspector]:[border-left:1px_solid_var(--ws-line)] [&_.ws-inspector]:[border:0] max-[1100px]:[&_.ws-inspector]:h-auto max-[1100px]:[&_.ws-inspector]:[overflow-y:visible] max-[1100px]:[&_.ws-inspector]:rounded-[14px] max-[1100px]:[&_.ws-inspector]:[border:1px_solid_var(--ws-line)] [&_.ws-list]:overflow-hidden [&_.ws-list]:rounded-[0_0_12px_12px] [&_.ws-list]:[border-bottom:1px_solid_var(--ws-line)] [&_.ws-list_.ws-capability-row:last-child]:[border-bottom:0] [&_.ws-results-caption]:min-h-[42px] [&_.ws-results-caption]:items-center [&_.ws-results-caption]:gap-3 [&_.ws-results-caption]:pt-[11px] [&_.ws-results-caption]:pr-[18px] [&_.ws-results-caption]:pb-[11px] [&_.ws-results-caption]:pl-[18px] [&_.ws-results-caption]:text-[11px] [&_.ws-results-caption]:leading-[1.6] [&_.ws-results-caption]:text-[color:#70796a] [&_.ws-results-caption]:[background:#f8f9f5] max-[760px]:[&_.ws-results-caption]:pt-2.5 max-[760px]:[&_.ws-results-caption]:pr-[13px] max-[760px]:[&_.ws-results-caption]:pb-2.5 max-[760px]:[&_.ws-results-caption]:pl-[13px] max-[760px]:[&_.ws-results-caption]:text-[10px] [&_.ws-results-caption_output]:shrink-0 [&_.ws-results-caption_output]:text-[12px] [&_.ws-results-caption_output]:text-[color:#48553b] max-[760px]:[&_.ws-results-caption>:last-child]:max-w-[150px] [&_.ws-row-description]:mt-[5px] [&_.ws-row-description]:[display:-webkit-box] [&_.ws-row-description]:overflow-hidden [&_.ws-row-description]:text-[12px] [&_.ws-row-description]:leading-[1.75] [&_.ws-row-description]:[white-space:normal] [&_.ws-row-description]:text-[color:#626e56] [&_.ws-row-description]:[-webkit-box-orient:vertical] [&_.ws-row-description]:[-webkit-line-clamp:2] max-[760px]:[&_.ws-row-description]:text-[12px] [&_.ws-row-info>strong]:flex-wrap [&_.ws-row-info>strong]:gap-2 [&_.ws-row-info>strong]:text-[14px] [&_.ws-row-info>strong]:leading-[1.5] [&_.ws-row-info>strong]:font-bold [&_.ws-row-info>strong]:text-[color:#303a27] max-[760px]:[&_.ws-row-info>strong]:text-[13px] [&_.ws-row-meta]:mt-[9px] [&_.ws-row-meta]:gap-2.5 [&_.ws-row-meta]:text-[11px] [&_.ws-row-meta]:text-[color:#768369] [&_.ws-row-meta>span:first-child]:[border-color:#dfe6d4] [&_.ws-row-meta>span:first-child]:pt-[3px] [&_.ws-row-meta>span:first-child]:pr-1.5 [&_.ws-row-meta>span:first-child]:pb-[3px] [&_.ws-row-meta>span:first-child]:pl-1.5 [&_.ws-row-meta>span:first-child]:text-[10px] [&_.ws-row-meta>span:first-child]:text-[color:#5a6d48] [&_.ws-row-meta>span:first-child]:[background:#f8faf4] [&_.ws-row-number]:w-5 [&_.ws-row-number]:text-[11px] [&_.ws-row-number]:text-[color:#7c886f] [&_.ws-row-price]:min-w-[73px] max-[760px]:[&_.ws-row-price]:min-w-[61px] max-[760px]:[&_.ws-row-price]:pt-0.5 [&_.ws-row-price_small]:mt-1.5 [&_.ws-row-price_small]:text-[10px] [&_.ws-row-price_small]:text-[color:#727e65] [&_.ws-row-price_strong]:text-[15px] [&_.ws-row-price_strong]:[font-weight:650] [&_.ws-row-price_strong]:text-[color:#35412b] max-[760px]:[&_.ws-row-price_strong]:text-[14px] [&_.ws-row-version]:text-[11px] [&_.ws-row-version]:text-[color:#748166] [&_.ws-search-area]:rounded-[14px_14px_0_0] [&_.ws-search-area]:[border-color:#dedfd8] [&_.ws-search-area_input]:h-9 [&_.ws-search-area_input]:text-[14px] [&_.ws-search-area_input]:leading-[1.5] [&_.ws-search-area_input]:text-[color:#2d3328] max-[760px]:[&_.ws-search-area_input]:text-[16px] [&_.ws-search-area_input::placeholder]:text-[color:#72786b] [&_.ws-search-area_kbd]:rounded-[5px] [&_.ws-search-area_kbd]:pt-1 [&_.ws-search-area_kbd]:pr-1.5 [&_.ws-search-area_kbd]:pb-1 [&_.ws-search-area_kbd]:pl-1.5 [&_.ws-search-area_kbd]:[font-family:inherit] [&_.ws-search-area_kbd]:text-[11px] [&_.ws-search-area_kbd]:text-[color:#6c7461] [&_.ws-search-area_kbd]:[background:#fff] [&_.ws-search-area_kbd]:[border:1px_solid_#e0e3d9] [&_.ws-search-area_search_form]:mt-4 [&_.ws-search-area_search_form]:mr-4 [&_.ws-search-area_search_form]:mb-4 [&_.ws-search-area_search_form]:ml-4 [&_.ws-search-area_search_form]:min-h-[50px] [&_.ws-search-area_search_form]:gap-3 [&_.ws-search-area_search_form]:rounded-[10px] [&_.ws-search-area_search_form]:[border-color:#d9ddd2] [&_.ws-search-area_search_form]:pt-1.5 [&_.ws-search-area_search_form]:pr-[7px] [&_.ws-search-area_search_form]:pb-1.5 [&_.ws-search-area_search_form]:pl-[15px] [&_.ws-search-area_search_form]:text-[color:#626b58] [&_.ws-search-area_search_form]:[background:#fcfcfa] max-[760px]:[&_.ws-search-area_search_form]:mt-3 max-[760px]:[&_.ws-search-area_search_form]:mr-3 max-[760px]:[&_.ws-search-area_search_form]:mb-3 max-[760px]:[&_.ws-search-area_search_form]:ml-3 max-[760px]:[&_.ws-search-area_search_form]:gap-[9px] max-[760px]:[&_.ws-search-area_search_form]:pl-3 [&_.ws-search-area_search_form:focus-within]:[border-color:#7c8c68] [&_.ws-search-area_search_form:focus-within]:[box-shadow:0_0_0_3px_#eaf0e1] [&_.ws-search-area_search_form:focus-within]:[background:#fff] [&_.ws-search-submit]:h-9 [&_.ws-search-submit]:w-9 [&_.ws-search-submit]:rounded-[8px] [&_.ws-search-submit]:[border-color:#30382a] [&_.ws-search-submit]:text-[color:#fff] [&_.ws-search-submit]:[background:#30382a] [&_.ws-search-submit:hover]:[background:#49573d] [&_.ws-sort]:ml-auto [&_.ws-sort]:flex [&_.ws-sort]:items-center [&_.ws-sort]:gap-[7px] [&_.ws-sort]:text-[11px] [&_.ws-sort]:text-[color:#707968] max-[760px]:[&_.ws-sort]:[align-self:flex-end] [&_.ws-sort_select]:min-h-[34px] [&_.ws-sort_select]:max-w-[none] [&_.ws-sort_select]:rounded-[6px] [&_.ws-sort_select]:pt-[5px] [&_.ws-sort_select]:pr-[7px] [&_.ws-sort_select]:pb-[5px] [&_.ws-sort_select]:pl-[7px] [&_.ws-sort_select]:text-[11px] [&_.ws-sort_select]:text-[color:#49543e] [&_.ws-sort_select]:[background:#fff] [&_.ws-sort_select]:[border:1px_solid_#e2e5db] [&_.ws-topbar]:sticky [&_.ws-topbar]:top-0 [&_.ws-topbar]:z-20" : ""}`}
    >
      <a
        className="ws-skip fixed top-[-80px] left-5 z-100 rounded-[12px] bg-[#222] px-5 py-3 text-white [&:focus]:top-3"
        href="#workspace-content"
      >
        Skip to workspace
      </a>
      {mobileOpen && (
        <button
          type="button"
          className="ws-nav-backdrop hidden cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45] max-[760px]:fixed max-[760px]:top-0 max-[760px]:right-0 max-[760px]:bottom-0 max-[760px]:left-0 max-[760px]:z-49 max-[760px]:block max-[760px]:bg-[#22320f40] max-[760px]:[backdrop-filter:blur(2px)] max-[760px]:[border:0]"
          aria-label="Close navigation"
          onClick={() => setMobileOpen(false)}
        />
      )}
      <aside
        className={`ws-sidebar sticky top-0 flex h-dvh flex-col overflow-y-auto pt-[25px] pr-3 pb-0 pl-3 [background:#fff] [border-right:1px_solid_var(--ws-line)] max-[760px]:invisible max-[760px]:fixed max-[760px]:top-0 max-[760px]:bottom-0 max-[760px]:left-0 max-[760px]:z-50 max-[760px]:h-dvh max-[760px]:w-[235px] max-[760px]:[transform:translateX(-110%)] max-[760px]:pt-[22px] max-[760px]:[box-shadow:8px_0_30px_#22320f15] max-[760px]:[transition:transform_0.2s] max-[760px]:[&.is-open]:visible max-[760px]:[&.is-open]:[transform:translateX(0)] [&>nav]:mt-7 ${mobileOpen ? "is-open" : ""}`}
        aria-label="Workspace navigation"
      >
        <div className="ws-brand flex items-center gap-[9px] px-2.5 py-0">
          <Brand small />
          <span className="ws-beta mt-1 text-[10px] font-bold tracking-[0.06em] text-[#686f60]">
            BETA
          </span>
          <button
            type="button"
            className="ws-icon-button ws-menu-close hidden h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] max-[760px]:ml-auto max-[760px]:inline-flex max-[760px]:h-10 max-[760px]:w-10 [&:hover]:bg-[#edeee7]"
            aria-label="Close navigation"
            onClick={() => setMobileOpen(false)}
          >
            <Icon name="close" size={18} />
          </button>
        </div>
        <nav>
          {["Workspace", "Account"].map((group) => (
            <div
              className="ws-nav-group mb-[21px] [&_kbd]:ml-auto [&_kbd]:[font-family:inherit] [&_kbd]:text-[10px] [&_kbd]:text-[#686f60] [&>.ws-overline]:mx-3 [&>.ws-overline]:mt-0 [&>.ws-overline]:mb-[7px] [&>.ws-overline]:block [&>button]:flex [&>button]:min-h-10 [&>button]:w-full [&>button]:items-center [&>button]:gap-2.5 [&>button]:rounded-[10px] [&>button]:bg-transparent [&>button]:px-[11px] [&>button]:py-2 [&>button]:text-left [&>button]:text-[12px] [&>button]:[font-weight:550] [&>button]:text-[#686f60] [&>button]:[border:1px_solid_transparent] max-[760px]:[&>button]:min-h-[44px] [&>button:hover]:bg-[#f7f7f3] [&>button:hover]:text-[#262a22] [&>button[aria-current]]:[border-color:#e4e8da] [&>button[aria-current]]:bg-[#eff2e7] [&>button[aria-current]]:font-bold [&>button[aria-current]]:text-[#405039]"
              key={group}
            >
              <span className="ws-overline text-[10px] font-bold tracking-[0.13em] text-[#686f60] uppercase max-[760px]:text-[10px] max-[760px]:tracking-[0.1em]">
                {group}
              </span>
              {navigation
                .filter((item) => item.group === group)
                .map((item) => (
                  <button
                    type="button"
                    key={item.id}
                    aria-current={view === item.id ? "page" : undefined}
                    onClick={() => navigate(item.id)}
                    className="cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45]"
                  >
                    <Icon name={item.icon} size={17} />
                    <span>{item.label}</span>
                    {item.id === "discover" && <kbd>⌘ K</kbd>}
                  </button>
                ))}
            </div>
          ))}
        </nav>
        <div className="ws-sidebar-bottom mt-auto pt-5">
          <SidebarBalance
            token={token}
            credentialId={credential?.id}
            onOpen={() => navigate("settings", "balance")}
          />
          <Link
            href="/docs"
            className="ws-sidebar-link flex items-center gap-2 px-[9px] py-[19px] text-[11px] text-[color:var(--ws-muted)] [&_svg:last-child]:ml-auto"
          >
            <Icon name="book" size={15} />
            Docs &amp; guidance
            <Icon name="external" size={12} />
          </Link>
          <button
            className="ws-account flex w-full cursor-pointer items-center gap-2 bg-transparent px-0 py-4 text-left [border-top:1px_solid_var(--ws-line)] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&_small]:mt-1 [&_small]:block [&_small]:text-[10px] [&_small]:text-[color:var(--ws-muted)] [&_strong]:block [&_strong]:overflow-hidden [&_strong]:text-[10px] [&_strong]:text-ellipsis [&_strong]:whitespace-nowrap [&.ws-account-preview-note]:m-0 [&.ws-account-preview-note]:flex [&.ws-account-preview-note]:items-start [&.ws-account-preview-note]:gap-2 [&.ws-account-preview-note]:rounded-[0_0_12px_12px] [&.ws-account-preview-note]:bg-[#fafbf7] [&.ws-account-preview-note]:px-[var(--account-pad)] [&.ws-account-preview-note]:py-3 [&.ws-account-preview-note]:text-[11px] [&.ws-account-preview-note]:leading-[1.7] [&.ws-account-preview-note]:text-[color:var(--ws-muted)] [&.ws-account-preview-note]:[border-top:1px_solid_var(--ws-line)] [&.ws-account-preview-note_svg]:mt-0.5 [&.ws-account-preview-note_svg]:shrink-0 [&>span:nth-child(2)]:min-w-0 [&>span:nth-child(2)]:[flex:1]"
            type="button"
            onClick={() => navigate("settings")}
          >
            <span className="ws-avatar dark grid h-[29px] w-[29px] shrink-0 [place-items:center] rounded-[9px] bg-[#f3f5ed] text-[#686f60] [border:1px_solid_#e7e8df] [&_img]:rounded-full [&_img]:object-cover [&.dark]:rounded-full [&.dark]:[border-color:#2d302a] [&.dark]:bg-[#2d302a] [&.dark]:text-[#fff] [&.ws-avatar-actions]:mt-2.5 [&.ws-avatar-actions]:flex [&.ws-avatar-actions]:gap-3.5 [&.ws-avatar-actions_button]:text-[11px]">
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
      <div
        className="ws-main flex min-w-0 flex-col max-[760px]:min-h-dvh"
        inert={mobileOpen}
      >
        <header className="ws-topbar flex min-h-16 items-center justify-between gap-4 bg-white px-[26px] py-3 [border-bottom:1px_solid_var(--ws-line)] max-[1280px]:px-5 max-[1280px]:py-3 max-[760px]:min-h-[60px] max-[760px]:gap-2 max-[760px]:px-[15px] max-[760px]:py-2.5 max-[760px]:[&_.ws-button]:min-h-[34px] max-[760px]:[&_.ws-button]:text-[10px]">
          <div className="ws-breadcrumb flex items-center gap-3 text-[11px] text-[#686f60] max-[760px]:gap-[7px] max-[760px]:text-[10px] [&_strong]:font-semibold [&_strong]:text-[#52564a] max-[760px]:[&>span]:hidden max-[760px]:[&>svg]:hidden">
            <button
              type="button"
              className="ws-icon-button ws-mobile-menu hidden h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] max-[760px]:inline-flex max-[760px]:h-10 max-[760px]:w-10 [&:hover]:bg-[#edeee7]"
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
          <div className="ws-topbar-actions flex items-center gap-5 max-[1100px]:gap-3 max-[760px]:gap-2.5">
            <span className="ws-account-tier rounded-[7px] bg-[#f5f8ef] px-[11px] py-[5px] text-[12px] font-semibold text-[#44602b] capitalize [border:1px_solid_#d8dfcf]">
              {plan.data?.tier ?? "…"}
            </span>
            <span
              className="ws-account-credits text-[12px] text-[#6b7166]"
              title="Daily hosted build credits; separate from wallet funds"
            >
              {plan.data?.build_credits
                ? `${plan.data.build_credits.available} build credits`
                : "Build credits unavailable"}
            </span>
          </div>
        </header>
        <main
          id="workspace-content"
          className="ws-content min-w-0 [flex:1] px-[26px] pt-[27px] pb-9 max-[1280px]:px-5 max-[1280px]:py-[23px] max-[760px]:px-4 max-[760px]:py-6 min-[1700px]:p-8"
        >
          {view !== "discover" && (
            <div
              className={`ws-page-heading mb-6 flex items-center justify-between gap-5 max-[760px]:mb-5 max-[760px]:items-start max-[760px]:gap-3 [&_h1]:mt-2 [&_h1]:mr-0 [&_h1]:mb-[7px] [&_h1]:ml-0 [&_h1]:text-[clamp(22px,_1.8vw,_29px)] [&_h1]:leading-[1.25] [&_h1]:font-bold [&_h1]:tracking-[-0.055em] max-[760px]:[&_h1]:text-[24px] [&_p]:text-[11px] [&_p]:text-[color:#686f60] max-[760px]:[&_p]:text-[10px] max-[760px]:[&_p]:leading-[1.8] [&.ws-account-page-heading]:mb-[18px] [&.ws-account-page-heading_h1]:mt-0 [&.ws-account-page-heading_h1]:mr-0 [&.ws-account-page-heading_h1]:mb-0 [&.ws-account-page-heading_h1]:ml-0 [&.ws-account-page-heading_h1]:whitespace-nowrap [&.ws-discover-heading]:mb-[18px] max-[1100px]:[&>.ws-button]:text-[10px] max-[760px]:[&>.ws-button]:min-h-[35px] max-[760px]:[&>.ws-button]:w-[35px] max-[760px]:[&>.ws-button]:gap-0 max-[760px]:[&>.ws-button]:overflow-hidden max-[760px]:[&>.ws-button]:rounded-full max-[760px]:[&>.ws-button]:pt-2 max-[760px]:[&>.ws-button]:pr-2 max-[760px]:[&>.ws-button]:pb-2 max-[760px]:[&>.ws-button]:pl-2 max-[760px]:[&>.ws-button]:text-[0] max-[760px]:[&>.ws-button_svg]:w-4 ${view === "settings" ? "ws-account-page-heading" : ""}`}
            >
              <div>
                {view !== "settings" && (
                  <span className="ws-overline text-[10px] font-bold tracking-[0.13em] text-[#686f60] uppercase max-[760px]:text-[10px] max-[760px]:tracking-[0.1em]">
                    YOUR WORKSPACE
                  </span>
                )}
                <h1 className="font-display">{head.title}</h1>
                {view !== "settings" && <p>{head.description}</p>}
              </div>
            </div>
          )}
          {error && (
            <div
              className="ws-error-banner mb-[18px] flex flex-wrap items-center gap-[15px] rounded-[10px] bg-[#fff7ed] px-[15px] py-2.5 text-[11px] text-[#686f60] [border:1px_solid_#e8d5c3] [&>span]:[flex:1]"
              role="alert"
            >
              <span>{error}</span>
              <button
                type="button"
                className="ws-text-button inline-flex cursor-pointer items-center gap-[5px] px-0 py-1.5 text-[11px] font-semibold text-[#5f7651] [background:none] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:text-[#273d1c] [&:hover]:[text-decoration:underline]"
                onClick={refreshData}
              >
                Retry
              </button>
              <button
                type="button"
                className="ws-text-button inline-flex cursor-pointer items-center gap-[5px] px-0 py-1.5 text-[11px] font-semibold text-[#5f7651] [background:none] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:text-[#273d1c] [&:hover]:[text-decoration:underline]"
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
              className={`ws-discovery grid [grid-template-columns:minmax(0,_1fr)_330px] [align-items:start] gap-[22px] max-[1280px]:[grid-template-columns:minmax(0,_1fr)_310px] max-[1280px]:gap-4 max-[1100px]:[grid-template-columns:minmax(0,_1fr)] min-[1700px]:[grid-template-columns:minmax(0,_1fr)_370px] min-[1700px]:gap-7 ${detailOpen ? "detail-is-open max-[1100px]:[&_.ws-inspector]:[grid-row:1] max-[1100px]:[&_.ws-inspector]:block" : ""}`}
            >
              <section
                className="ws-catalog min-w-0"
                aria-label="Capability registry"
              >
                <h1 className="font-display sr-only">Discover</h1>
                <div className="ws-search-area rounded-[13px_13px_0_0] bg-white [border:1px_solid_var(--ws-line)] [&_input]:h-[31px] [&_input]:min-w-0 [&_input]:[flex:1] [&_input]:bg-transparent [&_input]:text-[11px] [&_input]:text-[#333b2b] [&_input]:[outline:0] [&_input]:[border:0] max-[760px]:[&_input]:text-[11px] [&_input::placeholder]:text-[#686f60] [&_kbd]:text-[10px] [&_kbd]:whitespace-nowrap [&_search_form]:m-3.5 [&_search_form]:flex [&_search_form]:items-center [&_search_form]:gap-2.5 [&_search_form]:rounded-[999px] [&_search_form]:bg-[#fafbf7] [&_search_form]:py-[5px] [&_search_form]:pr-1.5 [&_search_form]:pl-3 [&_search_form]:text-[#686f60] [&_search_form]:[border:1px_solid_#dfe1d7] max-[760px]:[&_search_form]:m-[11px] [&_search_form:focus-within]:[border-color:#87976f] [&_search_form:focus-within]:shadow-[0_0_0_3px_#e9efdf] max-[760px]:[&_search_kbd]:hidden">
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
                          className="ws-icon-button inline-flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:bg-[#edeee7]"
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
                        className="ws-search-submit grid h-[31px] w-[31px] shrink-0 cursor-pointer [place-items:center] rounded-full bg-[#edf1e3] text-[#686f60] [border:1px_solid_#e0e4d6] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:bg-[#dfe9cd]"
                        aria-label="Open first result"
                      >
                        <Icon name="arrow" size={16} />
                      </button>
                    </form>
                  </search>
                  <div className="ws-filterbar flex items-center justify-between gap-2.5 px-3.5 pt-0 pb-3 max-[1280px]:flex-col max-[1280px]:items-start max-[1280px]:gap-[5px] max-[1100px]:flex-row max-[1100px]:items-center max-[760px]:flex-col max-[760px]:items-start max-[760px]:gap-2 max-[760px]:px-2.5 max-[760px]:pt-0 max-[760px]:pb-2.5">
                    <div
                      className="ws-filters flex flex-wrap gap-[3px] max-[760px]:gap-0.5 [&_button]:flex [&_button]:min-h-[30px] [&_button]:items-center [&_button]:gap-[5px] [&_button]:rounded-[7px] [&_button]:px-2 [&_button]:py-[5px] [&_button]:text-[10px] [&_button]:whitespace-nowrap [&_button]:text-[#686f60] [&_button]:[background:none] [&_button]:[border:1px_solid_transparent] max-[760px]:[&_button]:min-h-[38px] max-[760px]:[&_button]:px-[7px] max-[760px]:[&_button]:py-1.5 max-[760px]:[&_button]:text-[10px] [&_button:hover]:bg-[#f4f5ee] [&_button>span]:text-[10px] [&_button>span]:opacity-[0.65] [&_button[aria-pressed='true']]:[border-color:#e3e7d9] [&_button[aria-pressed='true']]:bg-[#eff3e7] [&_button[aria-pressed='true']]:[font-weight:650] [&_button[aria-pressed='true']]:text-[#50633e]"
                      aria-label="Filter loaded capabilities by runtime"
                    >
                      {["all", ...Object.keys(runtimeLabels)].map((runtime) => (
                        <button
                          type="button"
                          aria-pressed={filter === runtime}
                          key={runtime}
                          onClick={() => setFilter(runtime)}
                          className="cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45]"
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
                    <label className="ws-sort max-[1280px]:[align-self:flex-end] max-[760px]:[align-self:flex-start] [&_select]:max-w-[110px] [&_select]:px-0 [&_select]:py-1 [&_select]:text-[10px] [&_select]:text-[#686f60] [&_select]:[background:none] [&_select]:[border:0]">
                      <span>Sort</span>
                      <select
                        value={sort}
                        aria-label="Sort loaded capabilities"
                        onChange={(e) => setSort(e.target.value)}
                        className="cursor-pointer"
                      >
                        <option value="newest">Newest first</option>
                        <option value="price">Lowest price</option>
                        <option value="name">Name A–Z</option>
                      </select>
                    </label>
                  </div>
                </div>
                <div className="ws-results-caption flex justify-between gap-2.5 bg-[#f4f5ef] px-[15px] py-3 text-[10px] text-[#686f60] [border-top:0] [border:1px_solid_var(--ws-line)] max-[760px]:px-3 max-[760px]:py-2.5 max-[760px]:text-[10px] [&>:first-child]:[font-weight:650] [&>:first-child]:text-[#636d55] max-[760px]:[&>:last-child]:max-w-[170px] max-[760px]:[&>:last-child]:text-right">
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
                <div
                  className="ws-list bg-white [border-left:1px_solid_var(--ws-line)] [border-right:1px_solid_var(--ws-line)]"
                  aria-busy={loading}
                >
                  {results.map((tool, index) => (
                    <button
                      className={
                        "cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45] " +
                        `ws-capability-row relative flex min-h-[92px] w-full items-center gap-3 pt-[17px] pr-[15px] pb-[17px] pl-[15px] text-left [background:transparent] [border-bottom:1px_solid_var(--ws-line)] [border:0] max-[1280px]:gap-[9px] max-[760px]:min-h-[92px] max-[760px]:gap-[9px] max-[760px]:pt-4 max-[760px]:pr-[11px] max-[760px]:pb-4 max-[760px]:pl-[11px] min-[1700px]:min-h-[96px] min-[1700px]:pt-5 min-[1700px]:pr-5 min-[1700px]:pb-5 min-[1700px]:pl-5 [&.is-selected]:[background:#f2f5e9] [&.is-selected::before]:absolute [&.is-selected::before]:top-4 [&.is-selected::before]:bottom-4 [&.is-selected::before]:left-[-1px] [&.is-selected::before]:w-[3px] [&.is-selected::before]:rounded-[2px] [&.is-selected::before]:[content:''] [&.is-selected::before]:[background:#839668] [&:hover]:[background:#f7f9f1] max-[760px]:[&>.ws-tool-mark]:h-[29px] max-[760px]:[&>.ws-tool-mark]:w-[29px] max-[760px]:[&>.ws-tool-mark]:rounded-[8px] max-[760px]:[&>.ws-tool-mark_svg]:w-4 ${selected?.id === tool.id ? "is-selected" : ""}`
                      }
                      key={tool.id}
                      type="button"
                      aria-pressed={selected?.id === tool.id}
                      onClick={() => openDetail(tool.id)}
                    >
                      <span className="ws-row-number w-[17px] shrink-0 text-[10px] text-[#686f60] [font-variant-numeric:tabular-nums] max-[1280px]:hidden">
                        {String(index + 1).padStart(2, "0")}
                      </span>
                      <span className="ws-row-info min-w-0 [flex:1] [&>strong]:flex [&>strong]:items-baseline [&>strong]:gap-[9px] [&>strong]:text-[12px] [&>strong]:font-bold [&>strong]:text-[#454d3a] max-[760px]:[&>strong]:gap-[5px] max-[760px]:[&>strong]:text-[11px] min-[1700px]:[&>strong]:text-[13px]">
                        <strong>
                          {tool.name}
                          <span className="ws-row-version text-[10px] font-normal text-[#686f60] max-[760px]:hidden">
                            v{tool.version}
                          </span>
                        </strong>
                        <span className="ws-row-description mt-1.5 block overflow-hidden text-[10px] leading-[1.6] text-ellipsis whitespace-nowrap text-[#686f60] max-[760px]:[display:-webkit-box] max-[760px]:overflow-hidden max-[760px]:text-[10px] max-[760px]:[white-space:normal] max-[760px]:[-webkit-box-orient:vertical] max-[760px]:[-webkit-line-clamp:2] min-[1700px]:text-[11px]">
                          {tool.description}
                        </span>
                        <span className="ws-row-meta mt-2 flex flex-wrap items-center gap-2 text-[10px] text-[#686f60] max-[760px]:gap-[5px] max-[760px]:text-[10px] [&>span:first-child]:rounded-[4px] [&>span:first-child]:bg-[#f7f8f2] [&>span:first-child]:px-[5px] [&>span:first-child]:py-0.5 [&>span:first-child]:text-[#686f60] [&>span:first-child]:[border:1px_solid_#e5e8dc] max-[760px]:[&>span:last-child]:hidden">
                          <span>{runtimeLabels[tool.runtime]}</span>
                          <PublisherSummary metadata={tool.metadata} />
                        </span>
                      </span>
                      <span className="ws-row-price min-w-[61px] shrink-0 text-right max-[760px]:min-w-[46px] [&_small]:mt-[5px] [&_small]:block [&_small]:text-[10px] [&_small]:text-[#686f60] max-[760px]:[&_small]:text-[9px] [&_strong]:text-[12px] [&_strong]:[font-weight:650] [&_strong]:text-[#5d684f] [&_strong]:[font-variant-numeric:tabular-nums] max-[760px]:[&_strong]:text-[11px]">
                        <strong>{atomicMoney(tool.price)}</strong>
                        <small>USDC / call</small>
                      </span>
                      <Icon
                        className="ws-row-chevron text-[#686f60] max-[760px]:hidden"
                        name="chevron"
                        size={15}
                      />
                    </button>
                  ))}
                  {!results.length && (
                    <EmptyState title="No matching capabilities">
                      Try another name or reset the runtime filter.
                      <button
                        className="ws-text-button inline-flex cursor-pointer items-center gap-[5px] px-0 py-1.5 text-[11px] font-semibold text-[#5f7651] [background:none] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:text-[#273d1c] [&:hover]:[text-decoration:underline]"
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
                      className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
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
                className="ws-inspector sticky top-5 min-w-0 overflow-hidden rounded-[15px] bg-white p-5 shadow-[0_4px_18px_#3b4f1404] [border:1px_solid_#e0e5d6] max-[1100px]:static max-[1100px]:hidden max-[1100px]:[scroll-margin-top:20px] max-[760px]:p-[19px] min-[1700px]:p-6 [&_.ws-button]:w-full [&_.ws-field_input]:text-[10px] max-[760px]:[&_.ws-field_input]:min-h-[42px] max-[760px]:[&_.ws-field_input]:text-[16px] [&_.ws-form]:gap-[13px] max-[1100px]:[&_.ws-form]:grid max-[1100px]:[&_.ws-form]:grid-cols-[1fr_1fr] max-[760px]:[&_.ws-form]:flex max-[1100px]:[&_.ws-form>:nth-child(n+3)]:[grid-column:1/-1] [&_.ws-inspector-price]:mx-0 [&_.ws-inspector-price]:my-[23px] [&_.ws-inspector-price_small]:text-[11px] [&_.ws-inspector-price_small]:text-[#68715e] [&_.ws-inspector-price>strong]:text-[24px] [&_.ws-inspector-price>strong]:text-[#32392b] [&_.ws-inspector-price>strong]:[font-variant-numeric:tabular-nums] [&_.ws-tool-title]:mb-3 [&_.ws-tool-title_h2]:text-[20px] [&_.ws-tool-title_h2]:leading-[1.45] [&_.ws-tool-title_h2]:tracking-[-0.04em] [&_.ws-tool-title_h2]:text-[#292d25] [&.ws-inspector-heading]:mb-[17px] [&.ws-inspector-heading]:flex [&.ws-inspector-heading]:items-center [&.ws-inspector-heading]:justify-between [&.ws-inspector-tabs]:flex [&.ws-inspector-tabs]:gap-[19px] [&.ws-inspector-tabs]:[border-bottom:1px_solid_var(--ws-line)] [&.ws-inspector-tabs_button]:px-0 [&.ws-inspector-tabs_button]:py-2.5 [&.ws-inspector-tabs_button]:text-[10px] [&.ws-inspector-tabs_button]:text-[#686f60] [&.ws-inspector-tabs_button]:[background:none] [&.ws-inspector-tabs_button]:[border-bottom:2px_solid_transparent] [&.ws-inspector-tabs_button]:[border:0] [&.ws-inspector-tabs_button[aria-pressed='true']]:[border-bottom-color:#768960] [&.ws-inspector-tabs_button[aria-pressed='true']]:[font-weight:650] [&.ws-inspector-tabs_button[aria-pressed='true']]:text-[#61764b] [&>.ws-tool-mark]:mb-[13px] [&>.ws-tool-mark]:h-[42px] [&>.ws-tool-mark]:w-[42px] [&>.ws-tool-mark]:rounded-[12px] [&>.ws-tool-mark]:bg-white"
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
          <footer className="ws-footer [&>:last-child]:font-display flex items-center justify-between gap-3 px-[27px] py-[17px] text-[10px] text-[#686f60] [border-top:1px_solid_var(--ws-line)] max-[760px]:p-[17px] [&>:last-child]:text-[15px] [&>:last-child]:[font-weight:800] [&>:last-child]:tracking-[-0.06em] [&>:last-child]:text-[#686f60]">
            <span>Built by agents. Used by agents.</span>
            <span>
              hive<span className="ws-footer-dot text-[#686f60]">.</span>
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
