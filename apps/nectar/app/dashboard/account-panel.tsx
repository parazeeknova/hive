"use client";

import Image from "next/image";
import { useRef, useState } from "react";
import type { FormEvent, KeyboardEvent } from "react";

import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";
import {
  preferenceFor,
  resolveAccountCapability,
} from "../../lib/account-store";
import type {
  AccountPreview,
  CapabilityPreference,
  Profile,
} from "../../lib/account-store";
import {
  apiOrigin,
  atomicMoney,
  errorMessage,
  parsePrice,
  runtimeLabels,
  shortId,
} from "../../lib/hiveway";
import type { Credential, RegistryCapability } from "../../lib/hiveway";
import { AccountBalance } from "./account-payments";
import { EmptyState, Json } from "./workspace-pages";

import "./account.css";

type UpdateAccount = (
  update: (current: AccountPreview) => AccountPreview
) => void;

const ProfileForm = ({
  profile,
  onSave,
}: {
  profile: Profile;
  onSave: (profile: Profile) => void;
}) => {
  const [changes, setChanges] = useState<Partial<Profile>>({});
  const draft = { ...profile, ...changes };
  const [message, setMessage] = useState("");
  const file = useRef<HTMLInputElement>(null);
  const save = (event: FormEvent) => {
    event.preventDefault();
    if (!draft.name.trim() || !/^[a-z\d_-]{3,30}$/u.test(draft.username)) {
      setMessage(
        "Choose a name and a username with 3–30 lowercase letters, numbers, underscores, or hyphens."
      );
      return;
    }
    onSave({ ...draft, name: draft.name.trim() });
    setChanges({});
    setMessage("Profile saved on this device.");
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
          <div className="ws-avatar-actions">
            <button
              type="button"
              className="ws-text-button"
              onClick={() => file.current?.click()}
            >
              Change photo
            </button>
            {draft.avatar && (
              <button
                type="button"
                className="ws-text-button"
                onClick={() => setChanges({ ...draft, avatar: "" })}
              >
                Remove
              </button>
            )}
          </div>
        </div>
        <input
          ref={file}
          type="file"
          className="sr-only"
          tabIndex={-1}
          aria-label="Upload profile photo"
          accept="image/png,image/jpeg,image/webp"
          onChange={(event) => {
            const image = event.target.files?.[0];
            event.target.value = "";
            if (!image) {
              return;
            }
            if (
              !["image/png", "image/jpeg", "image/webp"].includes(image.type) ||
              image.size > 1_000_000
            ) {
              setMessage(
                "Choose a PNG, JPEG, or WebP image smaller than 1 MB."
              );
              return;
            }
            const reader = new FileReader();
            reader.addEventListener(
              "load",
              () => {
                if (typeof reader.result === "string") {
                  setChanges((current) => ({
                    ...current,
                    avatar: reader.result as string,
                  }));
                  setMessage("Photo ready. Save your profile to keep it.");
                }
              },
              { once: true }
            );
            reader.addEventListener(
              "error",
              () => setMessage("Unable to read that photo. Try another image."),
              { once: true }
            );
            reader.readAsDataURL(image);
          }}
        />
      </div>
      <form onSubmit={save} className="ws-profile-form">
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
          <output>{message || "Your name and photo on the hive."}</output>
          <button type="submit" className="ws-button">
            Save profile
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
}: {
  checked: boolean;
  label: string;
  onChange: () => void;
}) => (
  <button
    type="button"
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
  preference,
  onUpdate,
}: {
  tool: RegistryCapability;
  preference: CapabilityPreference;
  onUpdate: (pref: CapabilityPreference) => void;
}) => {
  const [priceDraft, setPriceDraft] = useState<string | null>(null);
  const price = priceDraft ?? atomicMoney(preference.paidPrice);
  const [message, setMessage] = useState("");
  const save = (event: FormEvent) => {
    event.preventDefault();
    try {
      onUpdate({ ...preference, paidPrice: parsePrice(price) });
      setPriceDraft(null);
      setMessage("Price saved in this preview.");
    } catch (error) {
      setMessage(errorMessage(error));
    }
  };
  return (
    <>
      <div className="ws-author-row">
        <span className="ws-author-avatar" aria-hidden="true">
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
        <span>{preference.active ? "Enabled" : "Disabled"}</span>
      </div>
      <div className="ws-account-settings-section">
        <div className="ws-setting-row">
          <div>
            <h3>Marketplace listing</h3>
            <p>
              {preference.active
                ? "Available for agents to discover."
                : "Hidden from the marketplace."}
            </p>
          </div>
          <SettingSwitch
            label="Enable capability on marketplace"
            checked={preference.active}
            onChange={() => {
              onUpdate({ ...preference, active: !preference.active });
              setMessage("Marketplace preference saved in this preview.");
            }}
          />
        </div>
      </div>
      <div className="ws-account-settings-section">
        <div className="ws-setting-row">
          <div>
            <h3>Paid capability</h3>
            <p>
              {preference.paid
                ? "Earn each time your capability is invoked."
                : "Free for other agents to use."}
            </p>
          </div>
          <SettingSwitch
            label="Charge per invocation"
            checked={preference.paid}
            onChange={() => {
              onUpdate({ ...preference, paid: !preference.paid });
              setMessage("Pricing preference saved in this preview.");
            }}
          />
        </div>
        <form onSubmit={save} className="ws-capability-price-form">
          <label className="ws-field">
            Price per call · USDC
            <input
              inputMode="decimal"
              value={preference.paid ? price : "0.00"}
              disabled={!preference.paid}
              onChange={(event) => setPriceDraft(event.target.value)}
              required
              aria-describedby="account-price-note"
            />
          </label>
          <p id="account-price-note" className="ws-fine">
            {preference.paid
              ? "Up to six decimal places. Set the amount charged per invocation."
              : "No charge per invocation. Your previous paid price is kept for later."}
          </p>
          <button
            type="submit"
            className="ws-button ws-primary"
            disabled={!preference.paid}
          >
            Save price <Icon name="check" size={14} />
          </button>
        </form>
      </div>
      <output className="ws-account-edit-status">
        {message || "Select a setting to update your capability."}
      </output>
      <p className="ws-fine">
        These preferences are saved on this device. Live marketplace updates are
        not connected yet.
      </p>
    </>
  );
};

const AccountSession = ({
  credential,
  onConnect,
  onDisconnect,
}: {
  credential: Credential | null;
  onConnect: () => void;
  onDisconnect: () => void;
}) => (
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
            <dt>Wallet</dt>
            <dd>
              <code>{credential.wallet}</code>
            </dd>
          </div>
          <div>
            <dt>Expires</dt>
            <dd>{new Date(credential.expires_at * 1000).toLocaleString()}</dd>
          </div>
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
        </dl>
        <p className="ws-fine">
          Session authorization is a spending limit, separate from your account
          balance.
        </p>
        <button type="button" className="ws-button" onClick={onDisconnect}>
          <Icon name="logout" size={14} /> Disconnect session
        </button>
      </>
    ) : (
      <button type="button" className="ws-button" onClick={onConnect}>
        <Icon name="wallet" size={15} /> Connect wallet
      </button>
    )}
    <p className="ws-muted">
      Your agent creates and publishes capabilities through its MCP connection.
    </p>
    <Json
      value={{
        authorization: "Bearer <your credential>",
        url: `${apiOrigin}/mcp`,
      }}
    />
  </details>
);

const accountTabs: {
  id: "profile" | "balance" | "capabilities";
  label: string;
  icon: IconName;
}[] = [
  { icon: "settings", id: "profile", label: "Profile" },
  { icon: "wallet", id: "balance", label: "Agent balance" },
  { icon: "box", id: "capabilities", label: "Capabilities" },
];
export type AccountTab = (typeof accountTabs)[number]["id"];

const AccountPanel = ({
  account,
  tab,
  onTabChange,
  scope,
  onUpdate,
  credential,
  capabilities,
  loading,
  onRefresh,
  onConnect,
  onDisconnect,
}: {
  account: AccountPreview;
  tab: AccountTab;
  onTabChange: (tab: AccountTab) => void;
  scope: string;
  onUpdate: UpdateAccount;
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
  const items = capabilities.map((tool) =>
    resolveAccountCapability(tool, account.capabilities)
  );
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
        id="account-panel-profile"
        className="ws-account-tab-panel ws-account-split ws-account-profile-panel"
        role="tabpanel"
        aria-labelledby="account-tab-profile"
        tabIndex={0}
        hidden={tab !== "profile"}
      >
        <ProfileForm
          profile={account.profile}
          onSave={(profile) => onUpdate((current) => ({ ...current, profile }))}
        />
        <aside className="ws-account-secondary">
          <AccountSession
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
        {!account.welcomeDismissed && (
          <div className="ws-welcome-credit">
            <span className="ws-welcome-credit-icon">
              <Icon name="wallet" size={21} />
            </span>
            <div>
              <strong>Your first $50 is on us.</strong>
              <p>
                New accounts get $50 in starter credit. Put your agent to work.
              </p>
            </div>
            <button
              className="ws-icon-button"
              type="button"
              aria-label="Dismiss starter credit message"
              onClick={() =>
                onUpdate((current) => ({ ...current, welcomeDismissed: true }))
              }
            >
              <Icon name="close" size={16} />
            </button>
          </div>
        )}
        <div className="ws-account-split">
          <AccountBalance
            balance={account.balance}
            preview={scope === "preview"}
          />
          <aside
            className="ws-account-secondary"
            aria-labelledby="payment-connection-heading"
          >
            <div className="ws-account-section-title">
              <div>
                <h2 id="payment-connection-heading">Payment connection</h2>
                <p>Funding and incoming payments.</p>
              </div>
            </div>
            <div className="ws-account-provider">
              <span className="ws-payment-provider-icon">
                <Icon name="wallet" size={20} />
              </span>
              <div>
                <strong>Dodo Payments</strong>
                <small>Not connected</small>
              </div>
            </div>
            <dl className="ws-account-payment-facts">
              <div>
                <dt>Top ups</dt>
                <dd>Not connected</dd>
              </div>
              <div>
                <dt>Incoming payments</dt>
                <dd>Not connected</dd>
              </div>
              <div>
                <dt>Transfers</dt>
                <dd>Unavailable</dd>
              </div>
            </dl>
            <p className="ws-fine">
              Live checkout and transfers will be available once payments are
              connected. Demo top-ups only change your preview balance.
            </p>
          </aside>
        </div>
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
            <p className="ws-fine">
              {credential
                ? `Showing capabilities owned by ${shortId(credential.wallet)} from the loaded registry.`
                : "Example capabilities · Connect your wallet to load your agent’s work."}
            </p>
          </section>
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
              preference={preferenceFor(rawSelected, account.capabilities)}
              onUpdate={(pref) =>
                onUpdate((current) => ({
                  ...current,
                  capabilities: {
                    ...current.capabilities,
                    [rawSelected.id]: pref,
                  },
                }))
              }
            />
          ) : (
            <EmptyState title="Capability settings">
              Select a capability to manage its price and marketplace
              availability.
            </EmptyState>
          )}
        </aside>
      </div>
      <div className="ws-account-preview-note">
        <Icon name="help" size={14} />
        <span>
          {account.storageError
            ? "Browser storage is unavailable. Changes last for this visit only."
            : "Account preview · Changes are saved on this device."}
        </span>
      </div>
    </div>
  );
};

export default AccountPanel;
