"use client";

import { useQuery, useQueryClient } from "@tanstack/react-query";
import Image from "next/image";
import { useRef, useState } from "react";
import type { SubmitEvent, KeyboardEvent } from "react";

import { Icon } from "../../components/icon";
import type { IconName } from "../../components/icon";
import type { Profile, WalletBalance } from "../../lib/account";
import {
  publicApiOrigin as apiOrigin,
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
import HostedBuilder from "./hosted-builder";
import { PublisherSummary } from "./publisher";
import { EmptyState, Json } from "./workspace-pages";

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
  const save = async (event: SubmitEvent<HTMLFormElement>) => {
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
    <section
      className="ws-account-profile mb-7 pb-7 [border-bottom:1px_solid_var(--ws-line)]"
      aria-labelledby="profile-heading"
    >
      <div className="ws-account-section-title [&_h2>span]:font-body mb-[18px] flex items-center justify-between gap-3.5 max-[760px]:items-start max-[760px]:gap-2.5 [&_h2]:text-[16px] [&_h2]:[font-weight:650] [&_h2]:tracking-[-0.035em] [&_h2]:text-[#303a27] [&_h2>span]:ml-[5px] [&_h2>span]:text-[12px] [&_h2>span]:font-normal [&_h2>span]:text-[color:var(--ws-muted)] [&_p]:mt-1.5 [&_p]:text-[11px] [&_p]:leading-[1.7] [&_p]:text-[#727c67] [&>.ws-badge]:text-[10px] [&>.ws-badge]:whitespace-nowrap max-[760px]:[&>.ws-badge]:max-w-[115px] max-[760px]:[&>.ws-badge]:[white-space:normal]">
        <div>
          <h2 id="profile-heading" className="font-display">
            Profile details
          </h2>
          <p>Your identity on the hive.</p>
        </div>
      </div>
      <div className="ws-profile-identity mb-5 flex items-center gap-3.5 pb-5 [border-bottom:1px_solid_var(--ws-line)] [&_p]:mt-1 [&_p]:text-[12px] [&_p]:text-[#727c67]">
        <span className="ws-profile-avatar grid h-[56px] w-[56px] shrink-0 [place-items:center] overflow-hidden rounded-full bg-[#eff2e7] text-[21px] font-semibold text-[#59674d] [border:1px_solid_#dfe6d4] [&_img]:h-full [&_img]:w-full [&_img]:object-cover">
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
          <strong className="ws-profile-name block text-[15px] [font-weight:650] tracking-[-0.04em] text-[#303a27]">
            {profile.name}
          </strong>
          <p>@{profile.username}</p>
        </div>
      </div>
      <form
        onSubmit={(event) => {
          void save(event);
        }}
        className="ws-profile-form grid grid-cols-[1fr_1fr] gap-4 max-[760px]:grid-cols-[1fr] max-[760px]:gap-[15px] [&_.ws-field]:m-0"
      >
        <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
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
        <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
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
        <div className="ws-profile-save [grid-column:1_/_-1] flex items-center justify-between gap-4 pt-4 text-[11px] leading-[1.7] text-[color:var(--ws-muted)] [border-top:1px_solid_var(--ws-line)] max-[760px]:items-start [&_button]:shrink-0">
          <output>{message || "Your public profile on the hive."}</output>
          <button
            type="submit"
            className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
            disabled={pending}
          >
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
    className="ws-setting-switch relative h-[22px] w-[38px] shrink-0 cursor-pointer rounded-[99px] bg-[#e8eae3] p-[3px] [border:1px_solid_#d8ddcf] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:disabled]:[cursor:wait] [&:disabled]:opacity-[0.5] [&>span]:block [&>span]:h-3.5 [&>span]:w-3.5 [&>span]:[transform:translateX(0)] [&>span]:rounded-full [&>span]:bg-white [&>span]:shadow-[0_1px_2px_#0001] [&[aria-checked='true']]:[border-color:#697f53] [&[aria-checked='true']]:bg-[#788d62] [&[aria-checked='true']>span]:[transform:translateX(16px)] [@media_(prefers-reduced-motion:no-preference)]:[&>span]:[transition:transform_160ms_ease]"
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
      <div className="ws-author-row mb-[19px] flex items-center gap-[9px] [&_.ws-detail-close]:ml-auto">
        <span className="ws-author-avatar grid h-7 w-7 shrink-0 [place-items:center] rounded-full bg-[#edf0e8] text-[10px] font-bold text-[#52604a] [border:1px_solid_#dfe3d9]">
          <Icon name="box" size={16} />
        </span>
        <span className="ws-author-name min-w-0 overflow-hidden text-[12px] font-semibold text-ellipsis whitespace-nowrap text-[#4c5545]">
          Your capability
        </span>
        <span className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
          v{tool.version}
        </span>
      </div>
      <div className="ws-tool-title mb-2.5 flex items-baseline gap-2 [&_h2]:[flex:1] [&_h2]:text-[17px] [&_h2]:leading-[1.5] [&_h2]:[font-weight:650] [&_h2]:tracking-[-0.045em]">
        <h2 className="font-display">{tool.name}</h2>
      </div>
      <p className="ws-capability-description text-[12px] leading-[1.85] [overflow-wrap:anywhere] text-[#62695b]">
        {tool.description}
      </p>
      <div className="ws-capability-tags mt-3.5 flex flex-wrap gap-1.5 [&>span]:rounded-[5px] [&>span]:bg-[#f5f6f2] [&>span]:px-[7px] [&>span]:py-1 [&>span]:text-[11px] [&>span]:leading-[1.4] [&>span]:text-[#5d6653] [&>span]:[border:1px_solid_#e0e4da]">
        <span>{runtimeLabels[tool.runtime]}</span>
        <PublisherSummary metadata={tool.metadata} />
        <span>{tool.active ? "Enabled" : "Disabled"}</span>
      </div>
      <div className="ws-account-settings-section px-0 py-[18px] [border-bottom:1px_solid_var(--ws-line)]">
        <div className="ws-setting-row [&_h3]:font-body flex items-center justify-between gap-4 [&_h3]:text-[13px] [&_h3]:[font-weight:650] [&_h3]:text-[#3c4931] [&_p]:mt-1.5 [&_p]:text-[11px] [&_p]:leading-[1.8] [&_p]:text-[#727c67]">
          <div>
            <h3 className="font-display">Marketplace listing</h3>
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
      <div className="ws-account-settings-section px-0 py-[18px] [border-bottom:1px_solid_var(--ws-line)]">
        <div className="ws-setting-row [&_h3]:font-body flex items-center justify-between gap-4 [&_h3]:text-[13px] [&_h3]:[font-weight:650] [&_h3]:text-[#3c4931] [&_p]:mt-1.5 [&_p]:text-[11px] [&_p]:leading-[1.8] [&_p]:text-[#727c67]">
          <div>
            <h3 className="font-display">
              {BigInt(tool.price) > 0n ? "Paid capability" : "Free capability"}
            </h3>
            <p>{atomicMoney(tool.price)} USDC per invocation.</p>
          </div>
        </div>
        <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
          Prices and code are immutable. Publish a new capability to change the
          price.
        </p>
      </div>
      <output
        className="ws-account-edit-status mt-5 block text-[11px] leading-[1.8] text-[#647653]"
        aria-live="polite"
      >
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
    <details
      open
      className="ws-account-session [&_summary]:font-display [border-top:0] [&_.ws-button]:[margin-block:15px] [&_summary]:flex [&_summary]:items-center [&_summary]:justify-between [&_summary]:gap-[15px] [&_summary]:px-0 [&_summary]:pt-0 [&_summary]:pb-[18px] [&_summary]:text-[16px] [&_summary]:[font-weight:650] [&_summary]:tracking-[-0.035em] [&_summary]:text-[#626e56] [&_summary]:[list-style:none] [&_summary::-webkit-details-marker]:hidden [&_summary>span]:inline-flex [&_summary>span]:items-center [&_summary>span]:gap-[9px] [&>.ws-muted]:mt-[18px] [&>.ws-muted]:text-[11px] [&>.ws-muted]:leading-[1.8] [&[open]_summary>svg]:[transform:rotate(90deg)]"
    >
      <summary className="cursor-pointer">
        <span>
          <Icon name="code" size={16} /> Agent connection
        </span>
        <Icon name="chevron" size={14} />
      </summary>
      {credential ? (
        <>
          <dl className="ws-facts m-0 [&_code]:text-[10px] [&_dd]:m-0 [&_dd]:min-w-0 [&_dd]:text-right [&_dd]:[overflow-wrap:anywhere] [&_dt]:shrink-0 [&_dt]:text-[#686f60] [&>div]:flex [&>div]:justify-between [&>div]:gap-3.5 [&>div]:px-0 [&>div]:py-2.5 [&>div]:text-[10px] [&>div]:leading-[1.7] [&>div]:text-[#686f60] [&>div]:[border-bottom:1px_solid_#efefe9]">
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
          <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
            {walletConnected
              ? "Your spending authorization is separate from your wallet balance."
              : "Free capabilities are ready to use. A wallet is optional and only needed for payments."}
          </p>
          <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
            Create and remove agent credentials in the API keys tab.
          </p>
          <button
            type="button"
            className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
            onClick={onConnect}
          >
            <Icon name="wallet" size={14} />{" "}
            {walletConnected ? "Change wallet" : "Connect wallet (optional)"}
          </button>
          {token !== "session" && (
            <button
              type="button"
              className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
              onClick={onDisconnect}
            >
              <Icon name="logout" size={14} /> Disconnect wallet
            </button>
          )}
        </>
      ) : (
        <button
          type="button"
          className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
          onClick={onConnect}
        >
          <Icon name="wallet" size={15} /> Connect wallet (optional)
        </button>
      )}
      <form action="/api/auth/logout" method="POST">
        <button
          type="submit"
          className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
        >
          <Icon name="logout" size={14} /> Sign out of Hive
        </button>
      </form>
      <p className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
        Use the MCP endpoint with your agent. Free capabilities work with your
        GitHub account, including publishing free tools. Paid publishing and
        payments use a wallet connection.
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
    <div className="ws-account-settings min-w-0 rounded-[12px] bg-white [--account-pad:20px] [border:1px_solid_var(--ws-line)] max-[760px]:[--account-pad:16px] [&_.ws-button]:min-h-[38px] [&_.ws-button]:rounded-[9px] [&_.ws-button]:text-[12px] [&_.ws-field]:text-[12px] [&_.ws-field_input]:min-h-10 [&_.ws-field_input]:rounded-[8px] [&_.ws-field_input]:px-3 [&_.ws-field_input]:py-2.5 [&_.ws-field_input]:text-[13px] max-[760px]:[&_.ws-field_input]:text-[16px]">
      <div
        ref={tabs}
        className="ws-account-tabs mb-0 flex [scrollbar-width:thin] gap-6 overflow-x-auto px-[var(--account-pad)] py-0 [border-bottom:1px_solid_var(--ws-line)] max-[760px]:gap-5 [&>button]:relative [&>button]:inline-flex [&>button]:min-h-[46px] [&>button]:shrink-0 [&>button]:items-center [&>button]:justify-center [&>button]:gap-2 [&>button]:bg-transparent [&>button]:px-0 [&>button]:py-3 [&>button]:text-[13px] [&>button]:font-medium [&>button]:whitespace-nowrap [&>button]:text-[#797f71] [&>button]:[border:0] max-[760px]:[&>button]:text-[12px] [&>button:hover]:text-[#303a27] max-[760px]:[&>button>svg]:hidden [&>button[aria-selected='true']]:[font-weight:650] [&>button[aria-selected='true']]:text-[#303a27] [&>button[aria-selected='true']::after]:absolute [&>button[aria-selected='true']::after]:right-0 [&>button[aria-selected='true']::after]:bottom-[-1px] [&>button[aria-selected='true']::after]:left-0 [&>button[aria-selected='true']::after]:h-0.5 [&>button[aria-selected='true']::after]:rounded-[2px_2px_0_0] [&>button[aria-selected='true']::after]:bg-[#697f53] [&>button[aria-selected='true']::after]:[content:'']"
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
            className="cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45]"
          >
            <Icon name={icon} size={16} />
            {label}
          </button>
        ))}
      </div>
      <div
        id="account-panel-api-keys"
        className="ws-account-tab-panel [&[hidden]]:hidden"
        role="tabpanel"
        aria-labelledby="account-tab-api-keys"
        tabIndex={0}
        hidden={tab !== "api-keys"}
      >
        {tab === "api-keys" && <ApiKeys />}
      </div>
      <div
        id="account-panel-profile"
        className="ws-account-tab-panel ws-account-split ws-account-profile-panel grid grid-cols-[minmax(0,_1fr)_330px] items-stretch gap-0 max-[1100px]:grid-cols-[minmax(0,_1fr)_290px] max-[900px]:grid-cols-[minmax(0,_1fr)] min-[1700px]:grid-cols-[minmax(0,_1fr)_390px] [&_.ws-account-profile]:m-0 [&_.ws-account-profile]:rounded-none [&_.ws-account-profile]:p-[var(--account-pad)] [&_.ws-account-profile]:[background:none] [&_.ws-account-profile]:[border:0] max-[760px]:[&_.ws-account-profile]:p-[var(--account-pad)] [&_.ws-account-session]:p-0 [&[hidden]]:hidden"
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
              className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
              type="button"
              onClick={() => {
                void profile.refetch();
              }}
            >
              Retry
            </button>
          </EmptyState>
        )}
        <aside className="ws-account-secondary min-w-0 p-[var(--account-pad)] [border-left:1px_solid_var(--ws-line)] max-[900px]:[border-width:1px_0_0]">
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
        className="ws-account-tab-panel ws-account-balance-panel [&_.ws-account-balance]:m-0 [&_.ws-account-balance]:rounded-none [&_.ws-account-balance]:p-[var(--account-pad)] [&_.ws-account-balance]:[background:none] [&_.ws-account-balance]:[border:0] [&_.ws-welcome-credit]:mx-[var(--account-pad)] [&_.ws-welcome-credit]:mt-4 [&_.ws-welcome-credit]:mb-0 [&[hidden]]:hidden"
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
        className="ws-account-tab-panel ws-account-layout grid grid-cols-[minmax(0,_1fr)_330px] items-start gap-0 max-[1100px]:grid-cols-[minmax(0,_1fr)_290px] max-[900px]:grid-cols-[minmax(0,_1fr)] min-[1700px]:grid-cols-[minmax(0,_1fr)_390px] [&_.ws-account-capabilities]:m-0 [&_.ws-account-capabilities]:rounded-none [&_.ws-account-capabilities]:p-[var(--account-pad)] [&_.ws-account-capabilities]:[background:none] [&_.ws-account-capabilities]:[border:0] [&_.ws-account-inspector]:sticky [&_.ws-account-inspector]:top-5 [&_.ws-account-inspector]:block [&_.ws-account-inspector]:[scroll-margin-top:25px] [&_.ws-account-inspector]:rounded-none [&_.ws-account-inspector]:[border-width:0_0_0_1px] [&_.ws-account-inspector]:bg-transparent [&_.ws-account-inspector]:p-[var(--account-pad)] [&_.ws-account-inspector]:shadow-[none] max-[1100px]:[&_.ws-account-inspector]:static max-[1100px]:[&_.ws-account-inspector]:block max-[1100px]:[&_.ws-account-inspector]:[border-width:0_0_0_1px] max-[1100px]:[&_.ws-account-inspector]:p-[var(--account-pad)] max-[900px]:[&_.ws-account-inspector]:[border-width:1px_0_0] [&[hidden]]:hidden"
        role="tabpanel"
        aria-labelledby="account-tab-capabilities"
        tabIndex={0}
        hidden={tab !== "capabilities"}
      >
        <div className="ws-account-main min-w-0">
          <section
            className="ws-account-capabilities mb-7 pb-7 [border-bottom:1px_solid_var(--ws-line)]"
            aria-labelledby="my-capabilities-heading"
          >
            <div className="ws-account-section-title [&_h2>span]:font-body mb-[18px] flex items-center justify-between gap-3.5 max-[760px]:items-start max-[760px]:gap-2.5 [&_h2]:text-[16px] [&_h2]:[font-weight:650] [&_h2]:tracking-[-0.035em] [&_h2]:text-[#303a27] [&_h2>span]:ml-[5px] [&_h2>span]:text-[12px] [&_h2>span]:font-normal [&_h2>span]:text-[color:var(--ws-muted)] [&_p]:mt-1.5 [&_p]:text-[11px] [&_p]:leading-[1.7] [&_p]:text-[#727c67] [&>.ws-badge]:text-[10px] [&>.ws-badge]:whitespace-nowrap max-[760px]:[&>.ws-badge]:max-w-[115px] max-[760px]:[&>.ws-badge]:[white-space:normal]">
              <div>
                <h2 id="my-capabilities-heading" className="font-display">
                  My capabilities <span>{items.length}</span>
                </h2>
                <p>Created by your agent. Managed by you.</p>
              </div>
              {credential && (
                <button
                  className="ws-icon-button inline-flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:bg-[#edeee7]"
                  type="button"
                  aria-label="Refresh your capabilities"
                  disabled={loading}
                  onClick={onRefresh}
                >
                  <Icon name="activity" size={16} />
                </button>
              )}
            </div>
            <div className="ws-account-search flex min-h-12 items-center gap-2.5 rounded-[10px] bg-[#fcfcfa] px-3 py-1.5 text-[#727c67] [border:1px_solid_#d9ddd2] [&_input]:h-[34px] [&_input]:min-w-0 [&_input]:[flex:1] [&_input]:bg-transparent [&_input]:text-[13px] [&_input]:outline-none [&_input]:[border:0] max-[760px]:[&_input]:text-[16px] [&_input:focus-visible]:outline-none [&:focus-within]:[border-color:#7c8c68] [&:focus-within]:bg-white [&:focus-within]:shadow-[0_0_0_3px_#eaf0e1]">
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
                  className="ws-icon-button inline-flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:bg-[#edeee7]"
                  aria-label="Clear capability search"
                  onClick={() => setQuery("")}
                >
                  <Icon name="close" size={13} />
                </button>
              )}
            </div>
            <div
              className="ws-account-capability-list mt-4 [border-top:1px_solid_var(--ws-line)] [&_.ws-capability-row]:min-h-[105px] [&_.ws-capability-row]:gap-3.5 [&_.ws-capability-row]:px-3.5 [&_.ws-capability-row]:py-[18px] max-[760px]:[&_.ws-capability-row]:gap-2.5 max-[760px]:[&_.ws-capability-row]:px-[9px] max-[760px]:[&_.ws-capability-row]:py-[17px] [&_.ws-row-description]:mt-[5px] [&_.ws-row-description]:[display:-webkit-box] [&_.ws-row-description]:text-[12px] [&_.ws-row-description]:leading-[1.75] [&_.ws-row-description]:[white-space:normal] [&_.ws-row-description]:text-[#626e56] [&_.ws-row-description]:[-webkit-box-orient:vertical] [&_.ws-row-description]:[-webkit-line-clamp:2] [&_.ws-row-info>strong]:flex-wrap [&_.ws-row-info>strong]:text-[14px] [&_.ws-row-info>strong]:leading-[1.5] [&_.ws-row-info>strong]:text-[#303a27] max-[760px]:[&_.ws-row-info>strong]:text-[13px] max-[760px]:[&_.ws-row-price]:min-w-[62px] [&_.ws-row-price_strong]:text-[14px] [&_.ws-row-price_strong]:text-[#35412b] max-[760px]:[&_.ws-row-price_strong]:text-[13px]"
              aria-busy={loading}
            >
              {results.map((tool, index) => (
                <button
                  key={tool.id}
                  type="button"
                  className={
                    "cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45] " +
                    `ws-capability-row relative flex min-h-[92px] w-full items-center gap-3 pt-[17px] pr-[15px] pb-[17px] pl-[15px] text-left [background:transparent] [border-bottom:1px_solid_var(--ws-line)] [border:0] max-[1280px]:gap-[9px] max-[760px]:min-h-[92px] max-[760px]:gap-[9px] max-[760px]:pt-4 max-[760px]:pr-[11px] max-[760px]:pb-4 max-[760px]:pl-[11px] min-[1700px]:min-h-[96px] min-[1700px]:pt-5 min-[1700px]:pr-5 min-[1700px]:pb-5 min-[1700px]:pl-5 [&.is-selected]:[background:#f2f5e9] [&.is-selected::before]:absolute [&.is-selected::before]:top-4 [&.is-selected::before]:bottom-4 [&.is-selected::before]:left-[-1px] [&.is-selected::before]:w-[3px] [&.is-selected::before]:rounded-[2px] [&.is-selected::before]:[content:''] [&.is-selected::before]:[background:#839668] [&:hover]:[background:#f7f9f1] max-[760px]:[&>.ws-tool-mark]:h-[29px] max-[760px]:[&>.ws-tool-mark]:w-[29px] max-[760px]:[&>.ws-tool-mark]:rounded-[8px] max-[760px]:[&>.ws-tool-mark_svg]:w-4 ${selected?.id === tool.id ? "is-selected" : ""}`
                  }
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
                      <span
                        className={`ws-account-listing-state inline-flex items-center gap-[5px] text-[color:#8a9180] [&.is-active]:text-[color:#657b51] [&.is-active>span]:[background:#839769] [&>span]:h-[5px] [&>span]:w-[5px] [&>span]:rounded-full [&>span]:[background:#aaa] ${tool.active ? "is-active" : ""}`}
                      >
                        <span />
                        {tool.active ? "Enabled" : "Disabled"}
                      </span>
                    </span>
                  </span>
                  <span className="ws-row-price min-w-[61px] shrink-0 text-right max-[760px]:min-w-[46px] [&_small]:mt-[5px] [&_small]:block [&_small]:text-[10px] [&_small]:text-[#686f60] max-[760px]:[&_small]:text-[9px] [&_strong]:text-[12px] [&_strong]:[font-weight:650] [&_strong]:text-[#5d684f] [&_strong]:[font-variant-numeric:tabular-nums] max-[760px]:[&_strong]:text-[11px]">
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
                className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
                disabled={loading}
                onClick={onLoadMore}
              >
                Load more owned capabilities
              </button>
            )}
            <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
              {credential
                ? `Showing capabilities owned by ${shortId(credential.wallet)} including inactive listings.`
                : "Sign in to load your published capabilities."}
            </p>
          </section>
          {credential?.scopes.includes("capabilities:publish") && (
            <section className="ws-account-capabilities mb-7 pb-7 [border-bottom:1px_solid_var(--ws-line)]">
              <HostedBuilder token={token} />
              <CreatorTools token={token} onRefresh={onRefresh} />
            </section>
          )}
        </div>
        <aside
          ref={inspector}
          className="ws-inspector ws-account-inspector sticky top-5 min-w-0 overflow-hidden rounded-[15px] bg-white p-5 shadow-[0_4px_18px_#3b4f1404] [border:1px_solid_#e0e5d6] max-[1100px]:static max-[1100px]:hidden max-[1100px]:[scroll-margin-top:20px] max-[760px]:p-[19px] min-[1700px]:p-6 [&_.ws-button]:w-full [&_.ws-field_input]:text-[10px] max-[760px]:[&_.ws-field_input]:min-h-[42px] max-[760px]:[&_.ws-field_input]:text-[16px] [&_.ws-form]:gap-[13px] max-[1100px]:[&_.ws-form]:grid max-[1100px]:[&_.ws-form]:grid-cols-[1fr_1fr] max-[760px]:[&_.ws-form]:flex max-[1100px]:[&_.ws-form>:nth-child(n+3)]:[grid-column:1/-1] [&_.ws-inspector-price]:mx-0 [&_.ws-inspector-price]:my-[23px] [&_.ws-inspector-price_small]:text-[11px] [&_.ws-inspector-price_small]:text-[#68715e] [&_.ws-inspector-price>strong]:text-[24px] [&_.ws-inspector-price>strong]:text-[#32392b] [&_.ws-inspector-price>strong]:[font-variant-numeric:tabular-nums] [&_.ws-tool-title]:mb-3 [&_.ws-tool-title_h2]:text-[20px] [&_.ws-tool-title_h2]:leading-[1.45] [&_.ws-tool-title_h2]:tracking-[-0.04em] [&_.ws-tool-title_h2]:text-[#292d25] [&.ws-inspector-heading]:mb-[17px] [&.ws-inspector-heading]:flex [&.ws-inspector-heading]:items-center [&.ws-inspector-heading]:justify-between [&.ws-inspector-tabs]:flex [&.ws-inspector-tabs]:gap-[19px] [&.ws-inspector-tabs]:[border-bottom:1px_solid_var(--ws-line)] [&.ws-inspector-tabs_button]:px-0 [&.ws-inspector-tabs_button]:py-2.5 [&.ws-inspector-tabs_button]:text-[10px] [&.ws-inspector-tabs_button]:text-[#686f60] [&.ws-inspector-tabs_button]:[background:none] [&.ws-inspector-tabs_button]:[border-bottom:2px_solid_transparent] [&.ws-inspector-tabs_button]:[border:0] [&.ws-inspector-tabs_button[aria-pressed='true']]:[border-bottom-color:#768960] [&.ws-inspector-tabs_button[aria-pressed='true']]:[font-weight:650] [&.ws-inspector-tabs_button[aria-pressed='true']]:text-[#61764b] [&>.ws-tool-mark]:mb-[13px] [&>.ws-tool-mark]:h-[42px] [&>.ws-tool-mark]:w-[42px] [&>.ws-tool-mark]:rounded-[12px] [&>.ws-tool-mark]:bg-white"
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
