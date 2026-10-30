"use client";

import { useQuery, useQueryClient } from "@tanstack/react-query";
import Image from "next/image";
import { useRef, useState } from "react";
import type { FormEvent, KeyboardEvent } from "react";

import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";
import type { Profile, WalletBalance } from "../../lib/account";
import {
  apiOrigin,
  atomicMoney,
  errorMessage,
  hiveway,
  runtimeLabels,
  shortId,
} from "../../lib/hiveway";
import type { Credential, RegistryCapability } from "../../lib/hiveway";
import ApiKeys from "./account-api-keys";
import { AccountBalance } from "./account-payments";
import CreatorTools from "./creator-tools";
import { EmptyState, Json } from "./workspace-pages";

import "./account.css";

const ProfileForm = ({
  profile,
  onSave,
}: {
  profile: Profile;
  onSave: (profile: Profile) => Promise<void>;
}) => {
  const [changes, setChanges] = useState<Partial<Profile>>({});
  const draft = { ...profile, ...changes };
  const [message, setMessage] = useState("");
  const [pending, setPending] = useState(false);
  const save = async (event: FormEvent) => {
    event.preventDefault();
    if (!draft.name.trim() || !/^[a-z\d_-]{3,30}$/u.test(draft.username)) {
      setMessage(
        "Choose a name and a username with 3–30 lowercase letters, numbers, underscores, or hyphens."
      );
      return;
    }
    setPending(true);
    setMessage("");
    try {
      await onSave({ ...draft, name: draft.name.trim() });
      setChanges({});
      setMessage("Profile saved to your account.");
    } catch (error) {
      setMessage(errorMessage(error));
    }
    setPending(false);
  };
  return (
    <section className="ws-account-profile" aria-labelledby="profile-heading">
      <div className="ws-account-section-title">
        <div>
          <h2 id="profile-heading">Profile details</h2>
          <p>Your identity on the hive.</p>
        </div>
      </div>
      <div className="ws-profile-identity">
        <span className="ws-profile-avatar">
          {draft.avatar ? (
            <Image
              src={draft.avatar}
              alt=""
              width={64}
              height={64}
              unoptimized
            />
          ) : (
            draft.name
              .trim()
              .split(/\s+/u)
              .slice(0, 2)
              .map((part) => part.charAt(0))
              .join("")
              .toUpperCase() || "HM"
          )}
        </span>
        <div>
          <strong className="ws-profile-name">{profile.name}</strong>
          <p>@{profile.username}</p>
        </div>
      </div>
      <form
        onSubmit={(event) => {
          void save(event);
        }}
        className="ws-profile-form"
      >
        <label className="ws-field">
          Name
          <input
            value={draft.name}
            maxLength={60}
            required
            autoComplete="name"
            onChange={(event) =>
              setChanges({ ...draft, name: event.target.value })
            }
          />
        </label>
        <label className="ws-field">
          Username
          <input
            value={draft.username}
            maxLength={30}
            minLength={3}
            pattern="[a-z0-9_-]{3,30}"
            required
            autoComplete="username"
            onChange={(event) =>
              setChanges({
                ...draft,
                username: event.target.value.toLowerCase(),
              })
            }
          />
        </label>
        <div className="ws-profile-save">
          <output>{message || "Your public profile on the hive."}</output>
          <button type="submit" className="ws-button" disabled={pending}>
            {pending ? "Saving…" : "Save profile"}
          </button>
        </div>
      </form>
    </section>
  );
};

const SettingSwitch = ({
  checked,
  label,
  onChange,
  disabled,
}: {
  checked: boolean;
  label: string;
  onChange: () => void;
  disabled: boolean;
}) => (
  <button
    type="button"
    disabled={disabled}
    role="switch"
    aria-checked={checked}
    aria-label={label}
    className="ws-setting-switch"
    onClick={onChange}
  >
    <span />
  </button>
);

const CapabilitySettings = ({
  tool,
  token,
  onRefresh,
}: {
  tool: RegistryCapability;
  token: string;
  onRefresh: () => void;
}) => {
  const [message, setMessage] = useState("");
  const [pending, setPending] = useState(false);
  return (
    <>
      <div className="ws-author-row">
        <span className="ws-author-avatar">
          <Icon name="box" size={16} />
        </span>
        <span className="ws-author-name">Your capability</span>
        <span className="ws-muted">v{tool.version}</span>
      </div>
      <div className="ws-tool-title">
        <h2>{tool.name}</h2>
      </div>
      <p className="ws-capability-description">{tool.description}</p>
      <div className="ws-capability-tags">
        <span>{runtimeLabels[tool.runtime]}</span>
        <span>{tool.active ? "Enabled" : "Disabled"}</span>
      </div>
      <div className="ws-account-settings-section">
        <div className="ws-setting-row">
          <div>
            <h3>Marketplace listing</h3>
            <p>
              {tool.active
                ? "Available for agents to discover."
                : "Hidden from the marketplace."}
            </p>
          </div>
          <SettingSwitch
            label="Enable capability on marketplace"
            checked={tool.active}
            disabled={pending}
            onChange={async () => {
              setPending(true);
              setMessage("");
              try {
                await hiveway(
                  `/v1/capabilities/${tool.id}/availability`,
                  token,
                  {
                    body: JSON.stringify({ active: !tool.active }),
                    method: "POST",
                  }
                );
                setMessage("Marketplace listing updated.");
                onRefresh();
              } catch (error) {
                setMessage(errorMessage(error));
              }
              setPending(false);
            }}
          />
        </div>
      </div>
      <div className="ws-account-settings-section">
        <div className="ws-setting-row">
          <div>
            <h3>
              {BigInt(tool.price) > 0n ? "Paid capability" : "Free capability"}
            </h3>
            <p>{atomicMoney(tool.price)} USDC per invocation.</p>
          </div>
        </div>
        <p className="ws-fine">
          Prices and code are immutable. Publish a new capability to change the
          price.
        </p>
      </div>
      <output className="ws-account-edit-status" aria-live="polite">
        {message}
      </output>
    </>
  );
};

const AccountSession = ({
  token,
  credential,
  onConnect,
  onDisconnect,
}: {
  credential: Credential | null;
  token: string;
  onConnect: () => void;
  onDisconnect: () => void;
}) => {
  const walletConnected = Boolean(
    credential && !credential.wallet.startsWith("github:")
  );
  return (
    <details open className="ws-account-session">
      <summary>
        <span>
          <Icon name="code" size={16} /> Agent connection
        </span>
        <Icon name="chevron" size={14} />
      </summary>
      {credential ? (
        <>
          <dl className="ws-facts">
            <div>
              <dt>
                {credential.wallet.startsWith("github:")
                  ? "GitHub identity"
                  : "Wallet"}
              </dt>
              <dd>
                <code>
                  {walletConnected ? credential.wallet : `@${credential.name}`}
                </code>
              </dd>
            </div>
            <div>
              <dt>Expires</dt>
              <dd>{new Date(credential.expires_at * 1000).toLocaleString()}</dd>
            </div>
            {walletConnected && (
              <>
                <div>
                  <dt>Session limit</dt>
                  <dd>{atomicMoney(credential.total_limit)} USDC</dd>
                </div>
                <div>
                  <dt>Spent / reserved</dt>
                  <dd>
                    {atomicMoney(credential.spent)} /{" "}
                    {atomicMoney(credential.reserved)} USDC
                  </dd>
                </div>
              </>
            )}
          </dl>
          <p className="ws-fine">
            {walletConnected
              ? "Your spending authorization is separate from your wallet balance."
              : "Free capabilities are ready to use. A wallet is optional and only needed for payments."}
          </p>
          <p className="ws-fine">
            Create and remove agent credentials in the API keys tab.
          </p>
          <button type="button" className="ws-button" onClick={onConnect}>
            <Icon name="wallet" size={14} />{" "}
            {walletConnected ? "Change wallet" : "Connect wallet (optional)"}
          </button>
          {token !== "session" && (
            <button type="button" className="ws-button" onClick={onDisconnect}>
              <Icon name="logout" size={14} /> Disconnect wallet
            </button>
          )}
        </>
      ) : (
        <button type="button" className="ws-button" onClick={onConnect}>
          <Icon name="wallet" size={15} /> Connect wallet (optional)
        </button>
      )}
      <form action="/api/auth/logout" method="POST">
        <button type="submit" className="ws-button">
          <Icon name="logout" size={14} /> Sign out of Hive
        </button>
      </form>
      <p className="ws-muted">
        Use the MCP endpoint with your agent. Free capabilities work with your
        GitHub account; publishing and payments use a wallet connection.
      </p>
      <Json
        value={{
          authorization: "Bearer <your credential>",
          url: `${apiOrigin}/mcp`,
        }}
      />
    </details>
  );
};

const accountTabs: {
  id: "profile" | "balance" | "capabilities" | "api-keys";
  label: string;
  icon: IconName;
}[] = [
  { icon: "settings", id: "profile", label: "Profile" },
  { icon: "wallet", id: "balance", label: "Agent balance" },
  { icon: "box", id: "capabilities", label: "Capabilities" },
  { icon: "code", id: "api-keys", label: "API keys" },
];
export type AccountTab = (typeof accountTabs)[number]["id"];

const AccountPanel = ({
  hasMore,
  onLoadMore,
  token,
  tab,
  onTabChange,
  credential,
  capabilities,
  loading,
  onRefresh,
  onConnect,
  onDisconnect,
}: {
  hasMore: boolean;
  onLoadMore: () => void;
  token: string;
  tab: AccountTab;
  onTabChange: (tab: AccountTab) => void;
  credential: Credential | null;
  capabilities: RegistryCapability[];
  loading: boolean;
  onRefresh: () => void;
  onConnect: () => void;
  onDisconnect: () => void;
}) => {
  const tabs = useRef<HTMLDivElement>(null);
  const onTabKeyDown = (
    event: KeyboardEvent<HTMLButtonElement>,
    index: number
  ) => {
    let next = index;
    if (event.key === "ArrowRight") {
      next = (index + 1) % accountTabs.length;
    } else if (event.key === "ArrowLeft") {
      next = (index - 1 + accountTabs.length) % accountTabs.length;
    } else if (event.key === "Home") {
      next = 0;
    } else if (event.key === "End") {
      next = accountTabs.length - 1;
    } else {
      return;
    }
    event.preventDefault();
    const target = accountTabs[next];
    if (target) {
      onTabChange(target.id);
      tabs.current
        ?.querySelector<HTMLButtonElement>(`#account-tab-${target.id}`)
        ?.focus();
    }
  };
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState("");
  const inspector = useRef<HTMLElement>(null);
  const queryClient = useQueryClient();
  const profile = useQuery({
    queryFn: ({ signal }) =>
      hiveway<Profile>("/v1/account", "session", { signal }),
    queryKey: ["hive-account", "profile"],
  });
  const balance = useQuery({
    enabled: Boolean(token && credential),
    queryFn: ({ signal }) =>
      hiveway<WalletBalance>("/v1/account/balance", token, { signal }),
    queryKey: ["hive", credential?.id, "wallet-balance"],
  });
  const items = capabilities;
  const results = items.filter((tool) =>
    `${tool.name} ${tool.description}`
      .toLowerCase()
      .includes(query.toLowerCase())
  );
  const selected = items.find((tool) => tool.id === selectedId) ?? results[0];
  const rawSelected = capabilities.find((tool) => tool.id === selected?.id);
  return (
    <div className="ws-account-settings">
      <div
        ref={tabs}
        className="ws-account-tabs"
        role="tablist"
        aria-label="Account settings features"
      >
        {accountTabs.map(({ id, label, icon }, index) => (
          <button
            key={id}
            type="button"
            role="tab"
            id={`account-tab-${id}`}
            aria-controls={`account-panel-${id}`}
            aria-selected={tab === id}
            tabIndex={tab === id ? 0 : -1}
            onClick={() => onTabChange(id)}
            onKeyDown={(event) => onTabKeyDown(event, index)}
          >
            <Icon name={icon} size={16} />
            {label}
          </button>
        ))}
      </div>
      <div
        id="account-panel-api-keys"
        className="ws-account-tab-panel"
        role="tabpanel"
        aria-labelledby="account-tab-api-keys"
        tabIndex={0}
        hidden={tab !== "api-keys"}
      >
        {tab === "api-keys" && <ApiKeys />}
      </div>
      <div
        id="account-panel-profile"
        className="ws-account-tab-panel ws-account-split ws-account-profile-panel"
        role="tabpanel"
        aria-labelledby="account-tab-profile"
        tabIndex={0}
        hidden={tab !== "profile"}
      >
        {profile.data ? (
          <ProfileForm
            profile={profile.data}
            onSave={async (next) => {
              const saved = await hiveway<Profile>("/v1/account", "session", {
                body: JSON.stringify(next),
                method: "POST",
              });
              queryClient.setQueryData(["hive-account", "profile"], saved);
            }}
          />
        ) : (
          <EmptyState
            title={
              profile.error ? "Unable to load profile" : "Loading profile…"
            }
          >
            {profile.error ? errorMessage(profile.error) : ""}
            <button
              className="ws-button"
              type="button"
              onClick={() => {
                void profile.refetch();
              }}
            >
              Retry
            </button>
          </EmptyState>
        )}
        <aside className="ws-account-secondary">
          <AccountSession
            token={token}
            credential={credential}
            onConnect={onConnect}
            onDisconnect={onDisconnect}
          />
        </aside>
      </div>
      <div
        id="account-panel-balance"
        className="ws-account-tab-panel ws-account-balance-panel"
        role="tabpanel"
        aria-labelledby="account-tab-balance"
        tabIndex={0}
        hidden={tab !== "balance"}
      >
        <AccountBalance
          balance={balance.data ?? null}
          error={balance.error ? errorMessage(balance.error) : ""}
          loading={balance.isFetching}
          onRefresh={() => {
            void balance.refetch();
          }}
          onConnect={onConnect}
          token={token}
        />
      </div>
      <div
        id="account-panel-capabilities"
        className="ws-account-tab-panel ws-account-layout"
        role="tabpanel"
        aria-labelledby="account-tab-capabilities"
        tabIndex={0}
        hidden={tab !== "capabilities"}
      >
        <div className="ws-account-main">
          <section
            className="ws-account-capabilities"
            aria-labelledby="my-capabilities-heading"
          >
            <div className="ws-account-section-title">
              <div>
                <h2 id="my-capabilities-heading">
                  My capabilities <span>{items.length}</span>
                </h2>
                <p>Created by your agent. Managed by you.</p>
              </div>
              {credential && (
                <button
                  className="ws-icon-button"
                  type="button"
                  aria-label="Refresh your capabilities"
                  disabled={loading}
                  onClick={onRefresh}
                >
                  <Icon name="activity" size={16} />
                </button>
              )}
            </div>
            <div className="ws-account-search">
              <Icon name="search" size={17} />
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Search your capabilities…"
                aria-label="Search your capabilities"
              />
              {query && (
                <button
                  type="button"
                  className="ws-icon-button"
                  aria-label="Clear capability search"
                  onClick={() => setQuery("")}
                >
                  <Icon name="close" size={13} />
                </button>
              )}
            </div>
            <div className="ws-account-capability-list" aria-busy={loading}>
              {results.map((tool, index) => (
                <button
                  key={tool.id}
                  type="button"
                  className={`ws-capability-row ${selected?.id === tool.id ? "is-selected" : ""}`}
                  aria-pressed={selected?.id === tool.id}
                  onClick={() => {
                    setSelectedId(tool.id);
                    requestAnimationFrame(() => {
                      if (window.matchMedia("(max-width: 900px)").matches) {
                        inspector.current?.scrollIntoView({
                          behavior: window.matchMedia(
                            "(prefers-reduced-motion: reduce)"
                          ).matches
                            ? "instant"
                            : "smooth",
                          block: "start",
                        });
                      }
                    });
                  }}
                >
                  <span className="ws-row-number">
                    {String(index + 1).padStart(2, "0")}
                  </span>
                  <span className="ws-row-info">
                    <strong>
                      {tool.name}
                      <span className="ws-row-version">v{tool.version}</span>
                    </strong>
                    <span className="ws-row-description">
                      {tool.description}
                    </span>
                    <span className="ws-row-meta">
                      <span>{runtimeLabels[tool.runtime]}</span>
                      <span
                        className={`ws-account-listing-state ${tool.active ? "is-active" : ""}`}
                      >
                        <span />
                        {tool.active ? "Enabled" : "Disabled"}
                      </span>
                    </span>
                  </span>
                  <span className="ws-row-price">
                    <strong>
                      {BigInt(tool.price) === 0n
                        ? "Free"
                        : atomicMoney(tool.price)}
                    </strong>
                    <small>
                      {BigInt(tool.price) === 0n ? "No charge" : "USDC / call"}
                    </small>
                  </span>
                  <Icon name="chevron" size={15} />
                </button>
              ))}
              {!results.length && (
                <EmptyState
                  title={
                    query
                      ? "No matching capabilities"
                      : "Your agent builds. You manage."
                  }
                >
                  {query
                    ? "Try another name or clear the search."
                    : "Capabilities published by your agent will appear here."}
                </EmptyState>
              )}
            </div>
            {hasMore && (
              <button
                type="button"
                className="ws-button"
                disabled={loading}
                onClick={onLoadMore}
              >
                Load more owned capabilities
              </button>
            )}
            <p className="ws-fine">
              {credential
                ? `Showing capabilities owned by ${shortId(credential.wallet)} including inactive listings.`
                : "Connect your wallet to load your agent’s work."}
            </p>
          </section>
          {credential?.scopes.includes("capabilities:publish") && (
            <section className="ws-account-capabilities">
              <CreatorTools token={token} onRefresh={onRefresh} />
            </section>
          )}
        </div>
        <aside
          ref={inspector}
          className="ws-inspector ws-account-inspector"
          aria-label="Capability settings"
        >
          {rawSelected ? (
            <CapabilitySettings
              key={rawSelected.id}
              tool={rawSelected}
              token={token}
              onRefresh={onRefresh}
            />
          ) : (
            <EmptyState title="Capability settings">
              Select a capability to manage its price and marketplace
              availability.
            </EmptyState>
          )}
        </aside>
      </div>
    </div>
  );
};

export default AccountPanel;
