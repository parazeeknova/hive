"use client";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import type { ReactNode } from "react";

import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";
import { capabilityDisplayName } from "../../lib/capability-name";
import {
  atomicMoney,
  errorMessage,
  hiveway,
  runtimeLabels,
  shortId,
} from "../../lib/hiveway";
import type { RegistryCapability } from "../../lib/hiveway";
import { CapabilityArtwork } from "./capability-artwork";
import { IntegrationPanel } from "./integration-panel";
import { PublisherSummary } from "./publisher";
import { CapabilityActionDialog, Json } from "./workspace-pages";

import styles from "./capability-details.module.css";

const runtimeNotes = (runtime: RegistryCapability["runtime"]) => {
  if (runtime === "wasm") {
    return {
      limitation:
        "Execution is bounded by fuel, memory, input/output size, and schema validation.",
      output: "Output must match the published artifact output schema.",
    };
  }
  if (runtime === "connected_mcp") {
    return {
      limitation:
        "Requires your own online connection. Upstream quotas and charges apply to your account.",
      output:
        "The selected tool returns its actual MCP text, image or structured output. Tool schemas are discovered from your connected server.",
    };
  }
  if (runtime === "workflow") {
    return {
      limitation:
        "Every step uses the same repository input. Conditional steps and nested workflows are not supported.",
      output:
        "Output contains repository identity and a steps array with each capability, version, runtime, and result.",
    };
  }
  return {
    limitation:
      "Analysis describes declared repository manifests and may reach runtime analysis limits.",
    output:
      "Report fields can include repository_url, revision, manifests, technologies, dependencies, and analysis_limit_reached. The settled result is the source of truth.",
  };
};
const executionNote = (runtime: RegistryCapability["runtime"]) => {
  if (runtime === "connected_mcp") {
    return "Calls only your own connected MCP. Clickable HTML previews are served by Hive Local on your computer; the Hive server cannot start a server there.";
  }
  return runtime === "wasm"
    ? "Pure JSON-in/JSON-out Wasm. No filesystem, network, clock, or environment access."
    : "Public GitHub repositories only. Supply a full 40-character commit SHA; branches, tags, and private repositories are not supported.";
};
/** Connected integrations bind the caller's own connection; other runtimes use their own schema. */
const connectedInputSchema = (tool: RegistryCapability) =>
  tool.runtime === "connected_mcp"
    ? {
        properties: {
          arguments: { type: "object" },
          connection_id: { format: "uuid", type: "string" },
          tool: { enum: tool.metadata?.integration?.tools, type: "string" },
        },
        required: ["connection_id", "arguments"],
        type: "object",
      }
    : undefined;

const DetailSection = ({
  title,
  icon,
  children,
  open = false,
}: {
  title: string;
  icon: IconName;
  children: ReactNode;
  open?: boolean;
}) => (
  <details className={styles.section} open={open}>
    <summary>
      <span>
        <Icon name={icon} size={17} />
        {title}
      </span>
      <Icon name="plus" size={16} className={styles.sectionToggle ?? ""} />
    </summary>
    <div className={styles.sectionBody}>{children}</div>
  </details>
);

const priceNote = (tool: RegistryCapability) => {
  if (tool.metadata?.integration?.byok) {
    return "Hive routing is free. Your provider’s API usage and limits still apply.";
  }
  return BigInt(tool.price) === 0n
    ? "No payment required for this capability."
    : "Review the quote before authorizing a payment.";
};

const CapabilityLaunch = ({
  tool,
  onAction,
}: {
  tool: RegistryCapability;
  onAction: (mode: "sample" | "invoke") => void;
}) => {
  const free = BigInt(tool.price) === 0n;
  return (
    <div className={styles.launch}>
      <div className={styles.priceRow}>
        <div>
          <span className={styles.priceLabel}>
            {free
              ? "A head start, on the house."
              : "One capability. Every new task."}
          </span>
          <p className={styles.price}>
            {free ? (
              "Free to invoke"
            ) : (
              <>
                {atomicMoney(tool.price)} <small>USDC / call</small>
              </>
            )}
          </p>
        </div>
        <span className={styles.priceIcon}>
          <Icon name="bolt" size={22} />
        </span>
      </div>
      <div className={styles.actions}>
        <button
          className={styles.primary}
          type="button"
          onClick={() => onAction("sample")}
        >
          <Icon name="play" size={15} />
          Test capability
          <Icon name="arrow" size={16} />
        </button>
        <button
          className={styles.secondary}
          type="button"
          onClick={() => onAction("invoke")}
        >
          <Icon name="external" size={15} />
          Use in your agent
        </button>
      </div>
      <p className={styles.priceNote}>{priceNote(tool)}</p>
    </div>
  );
};

const copyLabels = { copied: "Copied", error: "Copy failed", idle: "Copy ID" };
const CapabilityFooter = ({ tool }: { tool: RegistryCapability }) => {
  const [copyState, setCopyState] = useState<"idle" | "copied" | "error">(
    "idle"
  );
  useEffect(() => {
    if (copyState === "idle") {
      return;
    }
    const timeout = setTimeout(() => setCopyState("idle"), 2000);
    return () => clearTimeout(timeout);
  }, [copyState]);
  const copyId = async () => {
    try {
      await navigator.clipboard.writeText(tool.id);
      setCopyState("copied");
    } catch {
      setCopyState("error");
    }
  };
  return (
    <footer className={styles.footer}>
      <span>
        Capability ID <code title={tool.id}>{shortId(tool.id)}</code>
      </span>
      <button
        type="button"
        onClick={() => {
          void copyId();
        }}
        aria-label="Copy capability ID"
      >
        <Icon name={copyState === "copied" ? "check" : "copy"} size={13} />
        <span aria-live="polite">{copyLabels[copyState]}</span>
      </button>
    </footer>
  );
};

const CapabilitySchema = ({
  tool,
  token,
}: {
  tool: RegistryCapability;
  token: string;
}) => {
  const { output: outputDescription } = runtimeNotes(tool.runtime);
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
  return (
    <DetailSection title="Input & output" icon="code">
      <p>
        {tool.runtime === "wasm"
          ? "The published artifact’s schemas."
          : "What this capability expects."}
      </p>
      {artifact.error && <p role="alert">{errorMessage(artifact.error)}</p>}
      <Json
        value={
          artifact.data ??
          connectedInputSchema(tool) ??
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
      <p>Actual execution results appear in Activity.</p>
    </DetailSection>
  );
};

export const CapabilityDetails = ({
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
  const { limitation: limitationDescription } = runtimeNotes(tool.runtime);
  const [action, setAction] = useState<"sample" | "invoke" | null>(null);
  const steps = Array.isArray(tool.definition.steps)
    ? tool.definition.steps.filter((id): id is string => typeof id === "string")
    : [];
  const integration = tool.metadata?.integration;
  const free = BigInt(tool.price) === 0n;
  const statLabel = integration ? "Tools" : "Invocations";
  const statValue = integration
    ? integration.tools.length
    : (tool.metadata?.invocation_count ?? "—");
  return (
    <article className={styles.card} aria-labelledby="capability-detail-title">
      <header className={styles.hero}>
        <div className={styles.eyebrow}>
          <span>
            <span className={styles.eyebrowDot} />
            Capability spotlight
          </span>
          <button
            className={styles.close}
            type="button"
            onClick={onClose}
            aria-label="Close capability details"
          >
            <Icon name="close" size={17} />
          </button>
        </div>
        <CapabilityArtwork tool={tool} />
        <div className={styles.heroCaption}>
          <span>{runtimeLabels[tool.runtime]}</span>
          <span>
            <Icon name="layers" size={13} />v{tool.version}
          </span>
        </div>
      </header>
      <div className={styles.body}>
        <div className={styles.titleRow}>
          <span className={styles.category}>
            {tool.metadata?.publisher.official
              ? "By the Hive team"
              : "From the capability network"}
          </span>
          <span className={styles.listing} data-active={tool.active}>
            <span />
            {tool.active ? "Published" : "Unlisted"}
          </span>
        </div>
        <h2
          id="capability-detail-title"
          className={`font-display ${styles.title}`}
        >
          {capabilityDisplayName(tool.name)}
        </h2>
        <p className={styles.description}>{tool.description}</p>
        <div className={styles.publisher}>
          <PublisherSummary metadata={tool.metadata} />
        </div>
        <div className={styles.facts}>
          <div>
            <Icon name="box" size={16} />
            <span>Execution</span>
            <strong>
              {tool.runtime === "connected_mcp"
                ? "Your connection"
                : "Hosted by Hive"}
            </strong>
          </div>
          <div>
            <Icon name="layers" size={16} />
            <span>{statLabel}</span>
            <strong>{statValue}</strong>
          </div>
          <div>
            <Icon name="wallet" size={16} />
            <span>Invocation</span>
            <strong>{free ? "Free" : `${atomicMoney(tool.price)} USDC`}</strong>
          </div>
        </div>
        <CapabilityLaunch tool={tool} onAction={setAction} />
        {integration && (
          <div className={styles.integration}>
            <IntegrationPanel integration={integration} token={token} />
          </div>
        )}
        <div className={styles.sections}>
          <CapabilitySchema tool={tool} token={token} />
          {steps.length > 0 && (
            <DetailSection
              title={`Included capabilities · ${steps.length}`}
              icon="compose"
            >
              <div className={styles.dependencies}>
                {steps.map((id, index) => {
                  const dependency = catalog.find((item) => item.id === id);
                  return (
                    <button
                      type="button"
                      key={id}
                      disabled={!dependency}
                      onClick={() => onSelect(id)}
                    >
                      <span className={styles.stepNumber}>
                        {String(index + 1).padStart(2, "0")}
                      </span>
                      <span>
                        {dependency
                          ? capabilityDisplayName(dependency.name)
                          : shortId(id)}
                      </span>
                      {dependency && (
                        <small>{atomicMoney(dependency.price)} USDC</small>
                      )}
                      <Icon name="arrow" size={14} />
                    </button>
                  );
                })}
              </div>
            </DetailSection>
          )}
          <DetailSection title="Execution & limits" icon="shield">
            <p>{executionNote(tool.runtime)}</p>
            <p>{limitationDescription}</p>
          </DetailSection>
          {!free && (
            <DetailSection title="Pricing & payment" icon="wallet">
              <p>
                {atomicMoney(tool.price)} USDC per invocation. Review the
                returned quote’s payment requirements before authorizing
                payment.
              </p>
              <p>Quoting alone does not submit a payment.</p>
            </DetailSection>
          )}
        </div>
        <CapabilityFooter tool={tool} />
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
    </article>
  );
};
