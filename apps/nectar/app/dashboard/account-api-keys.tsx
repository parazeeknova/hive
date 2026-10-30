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
    <section className="ws-api-keys" aria-labelledby="api-keys-heading">
      <div className="ws-account-section-title">
        <div>
          <h2 id="api-keys-heading">API keys</h2>
          <p>
            Connect your agents and tools to Hive. A wallet is optional for free
            capabilities and free publishing. Create a new key to grant
            publishing access to an agent.
          </p>
        </div>
        <button
          type="button"
          className="ws-icon-button"
          aria-label="Refresh API keys"
          disabled={keys.isFetching}
          onClick={refresh}
        >
          <Icon name="activity" size={16} />
        </button>
      </div>
      <form
        className="ws-api-key-create"
        onSubmit={async (event) => {
          event.preventDefault();
          setPending(true);
          setError("");
          setMessage("");
          try {
            setCreated(
              await hiveway<NewKey>("/v1/account/credentials", "session", {
                body: JSON.stringify({ name: name.trim() }),
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
        <label className="ws-field">
          Key name
          <input
            required
            maxLength={60}
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="e.g. Claude Code on my laptop"
          />
        </label>
        <button
          className="ws-button ws-primary"
          type="submit"
          disabled={pending || !name.trim()}
        >
          <Icon name="plus" size={14} /> Create API key
        </button>
      </form>
      <p className="ws-fine">
        Keys can run free capabilities and expire within one hour. Your browser
        login stays separate.
      </p>
      {created && (
        <div className="ws-api-key-secret">
          <strong>Save your new key</strong>
          <p>
            The full key is shown only now. Store it in your agent’s
            credentials.
          </p>
          <label className="ws-field" htmlFor="new-api-key">
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
          <div className="ws-api-key-actions">
            <button
              className="ws-button"
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
              className="ws-text-button"
              type="button"
              onClick={() => setCreated(null)}
            >
              Done, I saved it
            </button>
          </div>
        </div>
      )}
      {error && (
        <p className="ws-error" role="alert">
          {error}
        </p>
      )}
      {message && (
        <output className="ws-fine ws-credential-status">{message}</output>
      )}
      <div className="ws-api-key-list" aria-busy={keys.isFetching}>
        {keys.error && (
          <p className="ws-error" role="alert">
            {errorMessage(keys.error)}
          </p>
        )}
        {keys.isPending && <p className="ws-muted">Loading API keys…</p>}
        {keys.data?.keys.length === 0 && (
          <div className="ws-api-key-empty">
            <Icon name="code" size={22} />
            <h3>No API keys yet</h3>
            <p>Create a key to connect your first agent.</p>
          </div>
        )}
        {keys.data?.keys.map((key) => (
          <div className="ws-api-key-row" key={key.id}>
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
              className="ws-button"
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
      <p className="ws-fine">
        MCP endpoint: <code>{apiOrigin}/mcp</code>. Use your API key as the
        Bearer credential.
      </p>
    </section>
  );
};
export default ApiKeys;
