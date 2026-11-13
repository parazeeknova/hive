"use client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useId, useState } from "react";
import type { ReactNode } from "react";

import { Icon } from "../../components/icon";
import { LoadingState, Spinner } from "../../components/spinner";
import { HivewayError, errorMessage, hiveway } from "../../lib/hiveway";

interface SavedKey {
  hint: string;
  tools: number;
  last_error: string | null;
  updated_at: number;
  served_by: "hive" | "hive_local";
}
export interface ProviderStatus {
  slug: string;
  name: string;
  description: string;
  configured: boolean;
  key: SavedKey | null;
}
interface ProviderList {
  available: boolean;
  providers: ProviderStatus[];
}
export const providersQuery = ["hive-account", "providers"] as const;

const button =
  "ws-button inline-flex min-h-[34px] cursor-pointer items-center justify-center gap-2 rounded-[9px] bg-white px-3 py-[7px] text-[11px] [font-weight:650] text-[#3f4836] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover:not(:disabled)]:[border-color:#bbc1ae] [&:hover:not(:disabled)]:bg-[#f4f5ee]";
const primary =
  "ws-button ws-primary inline-flex min-h-[34px] cursor-pointer items-center justify-center gap-2 rounded-[9px] px-3 py-[7px] text-[11px] [font-weight:650] text-white [border:1px_solid_transparent] [background:linear-gradient(180deg,#333_0%,#151515_60%,#1b1b1b_100%)] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover:not(:disabled)]:brightness-[1.12]";
const input =
  "w-full min-w-0 rounded-[8px] bg-white px-[11px] py-2.5 font-mono text-[12px] text-[#4e5c40] [border:1px_solid_#dfe4d5] max-[760px]:text-[16px] focus-visible:outline-none focus-visible:[border-color:#7c8c68] focus-visible:shadow-[0_0_0_3px_#eaf0e1]";

const messages: Record<string, string> = {
  invalid_provider_key:
    "That does not look like an API key. Paste the key without spaces or line breaks.",
  provider_key_not_saved: "No key is saved for this provider yet.",
  provider_keys_not_configured:
    "Saving provider keys is not enabled on this Hive deployment.",
  provider_rate_limited:
    "The provider is rate limiting this key. Wait a moment and try again.",
  provider_rejected_key:
    "The provider rejected this key. Check that it is active and copied in full.",
  provider_returned_no_tools:
    "The provider accepted the key but returned no tools.",
  provider_unreachable:
    "Hive could not reach the provider. Your key was not saved; try again shortly.",
  unknown_provider: "This provider does not accept account keys.",
};
const describe = (error: unknown) => {
  if (error instanceof HivewayError) {
    if (error.status === 401) {
      return "Sign in to the dashboard to manage provider keys.";
    }
    return messages[error.message] ?? errorMessage(error);
  }
  return errorMessage(error);
};
const statusText = (provider: ProviderStatus) => {
  const { key } = provider;
  if (!key) {
    return "No key saved. Agents can use this provider after you add your own key.";
  }
  if (key.last_error === "rejected_by_provider") {
    return `The provider rejected key ${key.hint}. Replace it to restore the tools.`;
  }
  const served =
    key.served_by === "hive_local"
      ? "Hive Local is online and serves these tools from your computer."
      : "Hive calls the provider with your key.";
  return `Key ${key.hint} · ${key.tools} tools · ${served}`;
};

const KeyHeader = ({
  provider,
  action,
}: {
  provider: ProviderStatus;
  action: ReactNode;
}) => {
  const rejected = provider.key?.last_error === "rejected_by_provider";
  return (
    <div className="flex items-start justify-between gap-3">
      <div className="min-w-0">
        <strong className="flex items-center gap-2 text-[13px] text-[#303a27]">
          {provider.name}
          {provider.configured && (
            <span
              className={`rounded-[4px] px-[5px] py-0.5 text-[10px] font-medium ${rejected ? "bg-[#fbeee9] text-[#a45541]" : "bg-[#eef3e6] text-[#5f7651]"}`}
            >
              {rejected ? "Needs a new key" : "Active"}
            </span>
          )}
        </strong>
        <p className="mt-1 text-[11px] leading-[1.7] text-[#727c67]">
          {statusText(provider)}
        </p>
      </div>
      {action}
    </div>
  );
};

const KeyForm = ({
  provider,
  pending,
  busy,
  onSave,
  onCancel,
}: {
  provider: ProviderStatus;
  pending: boolean;
  busy: boolean;
  onSave: (key: string) => void;
  onCancel?: (() => void) | undefined;
}) => {
  const id = useId();
  const [value, setValue] = useState("");
  return (
    <form
      className="mt-3 flex flex-wrap items-end gap-2"
      onSubmit={(event) => {
        event.preventDefault();
        onSave(value.trim());
      }}
    >
      <label
        htmlFor={id}
        className="flex min-w-[200px] flex-1 flex-col gap-1.5 text-[10px] [font-weight:650] text-[#667655]"
      >
        {provider.configured ? "New API key" : "API key"}
        <input
          id={id}
          className={input}
          type="password"
          required
          maxLength={4096}
          autoComplete="off"
          spellCheck={false}
          placeholder={`Paste your ${provider.name} API key`}
          value={value}
          onChange={(event) => setValue(event.target.value)}
        />
      </label>
      <button
        type="submit"
        className={primary}
        disabled={busy || !value.trim()}
      >
        {pending ? <Spinner size={13} /> : <Icon name="shield" size={13} />}
        {pending ? "Verifying…" : "Verify and save"}
      </button>
      {onCancel && (
        <button
          type="button"
          className={button}
          disabled={busy}
          onClick={onCancel}
        >
          Cancel
        </button>
      )}
    </form>
  );
};

const KeyActions = ({
  busy,
  refreshing,
  removing,
  onRefresh,
  onRemove,
}: {
  busy: boolean;
  refreshing: boolean;
  removing: boolean;
  onRefresh: () => void;
  onRemove: () => void;
}) => {
  const [confirming, setConfirming] = useState(false);
  return (
    <div className="mt-3 flex flex-wrap items-center gap-2">
      <button
        type="button"
        className={button}
        disabled={busy}
        onClick={onRefresh}
      >
        {refreshing ? (
          <Spinner size={13} />
        ) : (
          <Icon name="activity" size={13} />
        )}
        Refresh tools
      </button>
      {confirming ? (
        <>
          <button
            type="button"
            className={`${button} text-[#a45541]`}
            disabled={busy}
            onClick={onRemove}
          >
            {removing && <Spinner size={13} />}
            Confirm remove
          </button>
          <button
            type="button"
            className={button}
            disabled={busy}
            onClick={() => setConfirming(false)}
          >
            Keep key
          </button>
        </>
      ) : (
        <button
          type="button"
          className={button}
          disabled={busy}
          onClick={() => setConfirming(true)}
        >
          Remove key
        </button>
      )}
    </div>
  );
};

/** Save, replace, refresh or remove one provider's key. */
export const ProviderKeyEditor = ({
  provider,
}: {
  provider: ProviderStatus;
}) => {
  const client = useQueryClient();
  const [editing, setEditing] = useState(!provider.configured);
  const [notice, setNotice] = useState("");
  const path = `/v1/account/providers/${encodeURIComponent(provider.slug)}`;
  const done = (text: string) => {
    setNotice(text);
    void client.invalidateQueries({ queryKey: providersQuery });
    // The account's connections (and so its MCP tools) changed.
    void client.invalidateQueries({
      predicate: (query) => query.queryKey.includes("connections"),
    });
  };
  const save = useMutation({
    mutationFn: (apiKey: string) =>
      hiveway<{ tools: number }>(path, "session", {
        body: JSON.stringify({ api_key: apiKey }),
        method: "PUT",
      }),
    onSuccess: (saved) => {
      setEditing(false);
      done(
        `Key verified and saved. ${saved.tools} ${provider.name} tools are available to your agents.`
      );
    },
  });
  const refresh = useMutation({
    mutationFn: () =>
      hiveway<{ tools: number }>(`${path}/refresh`, "session", {
        body: "{}",
        method: "POST",
      }),
    onSuccess: (saved) => done(`Tools refreshed · ${saved.tools} available.`),
  });
  const remove = useMutation({
    mutationFn: () => hiveway(path, "session", { method: "DELETE" }),
    onSuccess: () => {
      setEditing(true);
      done(`Key removed. ${provider.name} tools are no longer available.`);
    },
  });
  const busy = save.isPending || refresh.isPending || remove.isPending;
  const failure = save.error ?? refresh.error ?? remove.error;
  const start = (action: () => void) => {
    setNotice("");
    for (const mutation of [save, refresh, remove]) {
      mutation.reset();
    }
    action();
  };
  return (
    <div
      className="ws-provider-key rounded-[12px] bg-[#fbfcf8] p-4 [border:1px_solid_#e0e5d6]"
      aria-busy={busy}
    >
      <KeyHeader
        provider={provider}
        action={
          provider.configured &&
          !editing && (
            <button
              type="button"
              className={button}
              disabled={busy}
              onClick={() => start(() => setEditing(true))}
            >
              Replace key
            </button>
          )
        }
      />
      {editing && (
        <KeyForm
          provider={provider}
          pending={save.isPending}
          busy={busy}
          onSave={(key) => start(() => save.mutate(key))}
          onCancel={
            provider.configured
              ? () => start(() => setEditing(false))
              : undefined
          }
        />
      )}
      {provider.configured && (
        <KeyActions
          busy={busy}
          refreshing={refresh.isPending}
          removing={remove.isPending}
          onRefresh={() => start(() => refresh.mutate())}
          onRemove={() => start(() => remove.mutate())}
        />
      )}
      {failure && (
        <p
          className="mt-2 text-[11px] leading-[1.7] text-[#a45541]"
          role="alert"
        >
          {describe(failure)}
        </p>
      )}
      {notice && !failure && (
        <output className="mt-2 block text-[11px] leading-[1.7] text-[#5f7651]">
          {notice}
        </output>
      )}
    </div>
  );
};

export const useProviders = () =>
  useQuery({
    queryFn: ({ signal }) =>
      hiveway<ProviderList>("/v1/account/providers", "session", { signal }),
    queryKey: providersQuery,
    refetchInterval: 30_000,
    retry: (count, error) =>
      !(error instanceof HivewayError && error.status < 500) && count < 2,
  });

/** One provider's key editor, for its integration panel. */
export const ProviderKey = ({ slug }: { slug: string }) => {
  const providers = useProviders();
  const provider = providers.data?.providers.find((item) => item.slug === slug);
  if (providers.isPending) {
    return (
      <LoadingState label="Loading provider key…" className="min-h-[96px]" />
    );
  }
  if (providers.error) {
    return (
      <p className="text-[11px] leading-[1.7] text-[#a45541]" role="alert">
        {describe(providers.error)}
      </p>
    );
  }
  if (!providers.data?.available) {
    return <p>{messages.provider_keys_not_configured}</p>;
  }
  return provider ? <ProviderKeyEditor provider={provider} /> : null;
};

/** Every provider that accepts an account key. */
const ProviderKeys = () => {
  const providers = useProviders();
  return (
    <section
      className="ws-provider-keys p-6 pt-0"
      aria-labelledby="provider-keys-heading"
    >
      <div className="mb-4 pt-6 [border-top:1px_solid_var(--ws-line)]">
        <h2
          id="provider-keys-heading"
          className="font-display text-[16px] [font-weight:650] tracking-[-0.035em] text-[#303a27]"
        >
          Provider keys
        </h2>
        <p className="mt-1.5 text-[11px] leading-[1.7] text-[#727c67]">
          Add your own keys for external MCP providers. Hive verifies each key
          with the provider, stores it encrypted for this account only, and
          never shows it again. Your agents then get the provider’s tools
          through Hive.
        </p>
      </div>
      {providers.isPending && <LoadingState label="Loading provider keys…" />}
      {providers.error && (
        <div className="flex items-center gap-3" role="alert">
          <p className="text-[11px] leading-[1.7] text-[#a45541]">
            {describe(providers.error)}
          </p>
          <button
            type="button"
            className={button}
            onClick={() => {
              void providers.refetch();
            }}
          >
            Retry
          </button>
        </div>
      )}
      {providers.data && !providers.data.available && (
        <p className="text-[11px] leading-[1.7] text-[#727c67]">
          {messages.provider_keys_not_configured}
        </p>
      )}
      {providers.data?.available && (
        <div className="grid gap-3">
          {providers.data.providers.map((provider) => (
            <ProviderKeyEditor key={provider.slug} provider={provider} />
          ))}
        </div>
      )}
    </section>
  );
};
export default ProviderKeys;
