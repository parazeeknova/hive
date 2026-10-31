"use client";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { Icon } from "../../components/icon";
import {
  publicApiOrigin as apiOrigin,
  errorMessage,
  hiveway,
  shortId,
} from "../../lib/hiveway";

interface ApiKey {
  id: string;
  name: string;
  scopes: string[];
  created_at: number;
  expires_at: number;
  expired: boolean;
}
interface NewKey {
  id: string;
  name: string;
  token: string;
  expires_at: number;
}
const keysQuery = ["hive-account", "api-keys"] as const;
const ApiKeys = () => {
  const client = useQueryClient();
  const keys = useQuery({
    queryFn: ({ signal }) =>
      hiveway<{ keys: ApiKey[] }>("/v1/account/credentials", "session", {
        signal,
      }),
    queryKey: keysQuery,
    refetchInterval: 60_000,
  });
  const [name, setName] = useState("");
  const [expiry, setExpiry] = useState(86_400);
  const [created, setCreated] = useState<NewKey | null>(null);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [pending, setPending] = useState(false);
  const refresh = () => {
    void client.invalidateQueries({ queryKey: keysQuery });
  };
  const revoke = async (id: string) => {
    setPending(true);
    setError("");
    setMessage("");
    try {
      await hiveway(`/v1/account/credentials/${id}/revoke`, "session", {
        body: "{}",
        method: "POST",
      });
      if (created?.id === id) {
        setCreated(null);
      }
      refresh();
      setMessage("API key removed. It can no longer access Hive.");
    } catch (error_) {
      setError(errorMessage(error_));
    }
    setPending(false);
  };
  return (
    <section className="ws-api-keys p-6" aria-labelledby="api-keys-heading">
      <div className="ws-account-section-title [&_h2>span]:font-body mb-[18px] flex items-center justify-between gap-3.5 max-[760px]:items-start max-[760px]:gap-2.5 [&_h2]:text-[16px] [&_h2]:[font-weight:650] [&_h2]:tracking-[-0.035em] [&_h2]:text-[#303a27] [&_h2>span]:ml-[5px] [&_h2>span]:text-[12px] [&_h2>span]:font-normal [&_h2>span]:text-[color:var(--ws-muted)] [&_p]:mt-1.5 [&_p]:text-[11px] [&_p]:leading-[1.7] [&_p]:text-[#727c67] [&>.ws-badge]:text-[10px] [&>.ws-badge]:whitespace-nowrap max-[760px]:[&>.ws-badge]:max-w-[115px] max-[760px]:[&>.ws-badge]:[white-space:normal]">
        <div>
          <h2 id="api-keys-heading" className="font-display">
            API keys
          </h2>
          <p>
            Connect your agents and tools to Hive. A wallet is optional for free
            capabilities and free publishing. Create a new key to grant
            publishing access to an agent.
          </p>
        </div>
        <button
          type="button"
          className="ws-icon-button inline-flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:bg-[#edeee7]"
          aria-label="Refresh API keys"
          disabled={keys.isFetching}
          onClick={refresh}
        >
          <Icon name="activity" size={16} />
        </button>
      </div>
      <form
        className="ws-api-key-create mt-6 flex flex-wrap [align-items:end] gap-3 [&_.ws-field]:min-w-[180px] [&_.ws-field]:[flex:1]"
        onSubmit={async (event) => {
          event.preventDefault();
          setPending(true);
          setError("");
          setMessage("");
          try {
            setCreated(
              await hiveway<NewKey>("/v1/account/credentials", "session", {
                body: JSON.stringify({ expires_in: expiry, name: name.trim() }),
                method: "POST",
              })
            );
            setName("");
            refresh();
          } catch (error_) {
            setError(errorMessage(error_));
          }
          setPending(false);
        }}
      >
        <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
          Key name
          <input
            required
            maxLength={60}
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="e.g. Claude Code on my laptop"
          />
        </label>
        <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
          Expires after
          <select
            value={expiry}
            onChange={(event) => setExpiry(Number(event.target.value))}
            className="cursor-pointer"
          >
            <option value={3600}>1 hour</option>
            <option value={86_400}>1 day</option>
            <option value={604_800}>7 days</option>
            <option value={2_592_000}>30 days</option>
            <option value={7_776_000}>90 days</option>
          </select>
        </label>
        <button
          className="ws-button ws-primary inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#40473a] [&:hover]:bg-[#f4f5ee]"
          type="submit"
          disabled={pending || !name.trim()}
        >
          <Icon name="plus" size={14} /> Create API key
        </button>
      </form>
      <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
        Choose when each key expires. Keys can run free capabilities. Your
        browser login stays separate.
      </p>
      {created && (
        <div className="ws-api-key-secret mt-5 rounded-[12px] bg-[#f5f8f0] p-5 [border:1px_solid_#b8c5ab] [&_input]:font-mono [&>p]:mx-0 [&>p]:mt-2 [&>p]:mb-4 [&>p]:text-[12px] [&>p]:leading-[1.7] [&>p]:text-[#727c67]">
          <strong>Save your new key</strong>
          <p>
            The full key is shown only now. Store it in your agent’s
            credentials.
          </p>
          <label
            className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]"
            htmlFor="new-api-key"
          >
            {created.name}
            <input
              id="new-api-key"
              autoComplete="off"
              spellCheck={false}
              readOnly
              value={created.token}
              onFocus={(event) => event.target.select()}
            />
          </label>
          <div className="ws-api-key-actions mt-3.5 flex items-center gap-4">
            <button
              className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
              type="button"
              onClick={async () => {
                try {
                  await navigator.clipboard.writeText(created.token);
                  setMessage("API key copied.");
                } catch {
                  setError(
                    "Clipboard unavailable. Select and copy the key above."
                  );
                }
              }}
            >
              <Icon name="copy" size={14} /> Copy key
            </button>
            <button
              className="ws-text-button inline-flex cursor-pointer items-center gap-[5px] px-0 py-1.5 text-[11px] font-semibold text-[#5f7651] [background:none] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:text-[#273d1c] [&:hover]:[text-decoration:underline]"
              type="button"
              onClick={() => setCreated(null)}
            >
              Done, I saved it
            </button>
          </div>
        </div>
      )}
      {error && (
        <p
          className="ws-error mt-[13px] text-[11px] leading-[1.8] text-[#a45541]"
          role="alert"
        >
          {error}
        </p>
      )}
      {message && (
        <output className="ws-fine ws-credential-status mt-0 block text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
          {message}
        </output>
      )}
      <div
        className="ws-api-key-list mt-6 [border-top:1px_solid_var(--ws-line)]"
        aria-busy={keys.isFetching}
      >
        {keys.error && (
          <p
            className="ws-error mt-[13px] text-[11px] leading-[1.8] text-[#a45541]"
            role="alert"
          >
            {errorMessage(keys.error)}
          </p>
        )}
        {keys.isPending && (
          <p className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
            Loading API keys…
          </p>
        )}
        {keys.data?.keys.length === 0 && (
          <div className="ws-api-key-empty px-0 py-[30px] text-[#727c67] [&_h3]:mt-3 [&_h3]:text-[14px] [&_h3]:text-[#303a27] [&_p]:mx-0 [&_p]:mt-2 [&_p]:mb-4 [&_p]:text-[12px] [&_p]:leading-[1.7] [&_p]:text-[#727c67]">
            <Icon name="code" size={22} />
            <h3 className="font-display">No API keys yet</h3>
            <p>Create a key to connect your first agent.</p>
          </div>
        )}
        {keys.data?.keys.map((key) => (
          <div
            className="ws-api-key-row flex items-center justify-between gap-4 px-0 py-5 [border-bottom:1px_solid_var(--ws-line)] [&_.ws-button]:shrink-0 [&_p]:mt-[5px] [&_p]:block [&_p]:text-[11px] [&_p]:leading-[1.6] [&_p]:text-[#727c67] [&_small]:mt-[5px] [&_small]:block [&_small]:text-[11px] [&_small]:leading-[1.6] [&_small]:text-[#727c67] [&_strong]:text-[13px] [&>div]:min-w-0 [&>div]:[overflow-wrap:anywhere]"
            key={key.id}
          >
            <div>
              <strong>{key.name}</strong>
              <small>
                {shortId(key.id)} ·{" "}
                {new Date(key.created_at * 1000).toLocaleDateString()}
              </small>
              <p>
                {key.expired
                  ? "Expired"
                  : `Expires ${new Date(key.expires_at * 1000).toLocaleString()}`}
              </p>
            </div>
            <button
              type="button"
              className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
              disabled={pending}
              aria-label={`Remove API key ${key.name}`}
              onClick={() => {
                void revoke(key.id);
              }}
            >
              Remove key
            </button>
          </div>
        ))}
      </div>
      <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
        MCP endpoint: <code>{apiOrigin}/mcp</code>. Use your API key as the
        Bearer credential.
      </p>
    </section>
  );
};
export default ApiKeys;
