"use client";

import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";

import { Icon } from "../../components/icon";
import {
  publicApiOrigin as apiOrigin,
  atomicMoney,
  errorMessage,
  hiveway,
  issueWalletSession,
  shortId,
  terminalStates,
  validateRepository,
} from "../../lib/hiveway";
import type {
  Activity,
  Credential,
  Quote,
  Receipt,
  RegistryCapability,
  Sale,
} from "../../lib/hiveway";
import { sampleInput } from "../../lib/sample-input";

export const EmptyState = ({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) => (
  <div className="ws-empty">
    <Icon name="layers" size={25} />
    <h3>{title}</h3>
    <p>{children}</p>
  </div>
);
export const Json = ({ value }: { value: unknown }) => (
  <pre className="ws-json">{JSON.stringify(value, null, 2)}</pre>
);
export const Status = ({ state }: { state: string }) => {
  let appearance = "";
  if (state.includes("failed") || state === "expired") {
    appearance = "is-error";
  }
  if (state === "settled" || state === "succeeded") {
    appearance = "is-success";
  }
  return (
    <span className={`ws-status ${appearance}`}>
      <span />
      {state.replaceAll("_", " ")}
    </span>
  );
};

export const SessionDialog = ({
  onClose,
  onConnect,
}: {
  onClose: () => void;
  onConnect: (token: string, credential: Credential) => void;
}) => {
  const ref = useRef<HTMLDialogElement>(null);
  const [token, setToken] = useState("");
  const [creator, setCreator] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  const connect = async (event?: FormEvent) => {
    event?.preventDefault();
    setBusy(true);
    setError("");
    try {
      let issued = token.trim();
      if (!event) {
        issued = await issueWalletSession(creator);
      }
      const credential = await hiveway<Credential>(
        "/v1/credentials/me",
        issued
      );
      onConnect(issued, credential);
      onClose();
    } catch (error_) {
      setError(errorMessage(error_));
    }
    setBusy(false);
  };
  return (
    <dialog
      ref={ref}
      className="ws-dialog"
      onClose={onClose}
      aria-labelledby="session-title"
    >
      <div className="ws-dialog-heading">
        <div>
          <span className="ws-overline">YOUR WORKSPACE</span>
          <h2 id="session-title">Connect to the hive.</h2>
        </div>
        <button
          className="ws-icon-button"
          type="button"
          onClick={onClose}
          aria-label="Close connection dialog"
        >
          <Icon name="close" size={18} />
        </button>
      </div>
      <p>
        Sign a wallet challenge to browse live capabilities and manage your
        session.
      </p>
      <div className="ws-note">
        1 hour session · 1 USDC per call · 10 USDC total authorization. Signing
        in does not authorize a payment.
      </div>
      <label className="ws-checkbox">
        <input
          type="checkbox"
          checked={creator}
          onChange={(e) => setCreator(e.target.checked)}
        />
        Include creator sales access
      </label>
      <button
        type="button"
        className="ws-button ws-primary"
        disabled={busy}
        onClick={() => {
          void connect();
        }}
      >
        <Icon name="wallet" size={16} />
        {busy ? "Connecting…" : "Connect wallet"}
      </button>
      <details className="ws-disclosure">
        <summary>Use an existing scoped credential</summary>
        <form
          onSubmit={(event) => {
            void connect(event);
          }}
        >
          <label className="ws-field">
            Bearer token
            <input
              type="password"
              autoComplete="off"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="hwa_…"
              required
            />
          </label>
          <button
            className="ws-button"
            disabled={busy || !token.trim()}
            type="submit"
          >
            Connect credential
          </button>
        </form>
      </details>
      <p className="ws-fine">
        Credentials stay in memory and clear when this page reloads.
      </p>
      {error && (
        <p className="ws-error" role="alert">
          {error}
        </p>
      )}
    </dialog>
  );
};

export const ActivityPanel = ({
  token,
  activity,
  catalog,
  onRefresh,
}: {
  token: string;
  activity: Activity | null;
  catalog: RegistryCapability[];
  onRefresh: () => void;
}) => {
  const [receipt, setReceipt] = useState<Receipt | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const loadAbort = useRef<AbortController | null>(null);
  const pollAbort = useRef<AbortController | null>(null);
  useEffect(() => {
    const abort = () => {
      loadAbort.current?.abort();
      pollAbort.current?.abort();
    };
    if (!token) {
      abort();
    }
    return abort;
  }, [token]);
  const load = async (id: string, reconcile = false) => {
    loadAbort.current?.abort();
    pollAbort.current?.abort();
    const controller = new AbortController();
    loadAbort.current = controller;
    setBusy(true);
    setError("");
    try {
      const next = await hiveway<Receipt>(
        `/v1/invocations/${id}${reconcile ? "/reconcile" : ""}`,
        token,
        {
          signal: controller.signal,
          ...(reconcile ? { body: "{}", method: "POST" } : {}),
        }
      );
      if (controller.signal.aborted) {
        return;
      }
      setReceipt(next);
      if (reconcile) {
        onRefresh();
      }
    } catch (error_) {
      if (!controller.signal.aborted) {
        setError(errorMessage(error_));
      }
    }
    if (!controller.signal.aborted) {
      setBusy(false);
    }
  };
  const receiptId = receipt?.id;
  const receiptState = receipt?.state;
  useEffect(() => {
    if (!receiptId || !receiptState || terminalStates.has(receiptState)) {
      return;
    }
    const controller = new AbortController();
    pollAbort.current = controller;
    let timer: ReturnType<typeof setTimeout>;
    let disposed = false;
    let delay = 2000;
    const poll = async () => {
      if (!document.hidden) {
        try {
          const next = await hiveway<Receipt>(
            `/v1/invocations/${receiptId}`,
            token,
            { signal: controller.signal }
          );
          if (disposed || controller.signal.aborted) {
            return;
          }
          setReceipt(next);
          if (terminalStates.has(next.state)) {
            onRefresh();
            return;
          }
        } catch (error_) {
          if (!disposed && !controller.signal.aborted) {
            setError(errorMessage(error_));
          }
          return;
        }
      }
      delay = Math.min(delay * 1.5, 10_000);
      if (!disposed) {
        timer = setTimeout(() => {
          void poll();
        }, delay);
      }
    };
    timer = setTimeout(() => {
      void poll();
    }, delay);
    return () => {
      disposed = true;
      controller.abort();
      clearTimeout(timer);
    };
  }, [receiptId, receiptState, token, onRefresh]);
  return (
    <div className="ws-page-grid">
      <section className="ws-panel">
        <div className="ws-panel-title">
          <h2>Invocation history</h2>
          <button className="ws-text-button" type="button" onClick={onRefresh}>
            Refresh <Icon name="activity" size={14} />
          </button>
        </div>
        <p className="ws-muted">
          Up to 100 recent invocations for this credential.
        </p>
        {activity?.invocations.length ? (
          <div className="ws-history">
            {activity.invocations.map((item) => (
              <button
                key={item.id}
                type="button"
                className="ws-history-row"
                onClick={() => {
                  void load(item.id);
                }}
              >
                <span>
                  <strong>
                    {catalog.find((tool) => tool.id === item.capability_id)
                      ?.name ?? shortId(item.capability_id)}
                  </strong>
                  <small>
                    {new Date(item.created_at * 1000).toLocaleString()} ·{" "}
                    {item.payment_method ?? "Unpaid"}
                  </small>
                </span>
                <Status state={item.state} />
                <span className="ws-numeric">
                  {atomicMoney(item.price)}
                  <small>USDC</small>
                </span>
                <Icon name="chevron" size={14} />
              </button>
            ))}
          </div>
        ) : (
          <EmptyState title="Your next run starts here.">
            Quote a capability from Discover. Receipts and results will appear
            here.
          </EmptyState>
        )}
      </section>
      <section className="ws-panel">
        <div className="ws-panel-title">
          <h2>Receipt &amp; result</h2>
        </div>
        {receipt ? (
          <>
            <div className="ws-receipt-meta">
              <Status state={receipt.state} />
              <code>{receipt.id}</code>
            </div>
            {receipt.error_code && (
              <p className="ws-error">
                {receipt.error_code.replaceAll("_", " ")}
              </p>
            )}
            {receipt.payment_issue && (
              <p className="ws-note">
                Payment: {receipt.payment_issue.replaceAll("_", " ")}
              </p>
            )}
            {receipt.state === "settlement_unknown" ||
            receipt.state === "settling" ? (
              <p className="ws-note">
                Payment finality is pending. Keep this invocation; do not pay
                again.
              </p>
            ) : null}
            {receipt.state === "settled" ? (
              <>
                <Json value={receipt.result} />
                {receipt.settlement && (
                  <p className="ws-fine">
                    {receipt.settlement.network}
                    <br />
                    <code>{receipt.settlement.transaction}</code>
                  </p>
                )}
              </>
            ) : (
              <button
                type="button"
                className="ws-button"
                disabled={busy}
                onClick={() => {
                  void load(receipt.id, true);
                }}
              >
                {busy ? "Checking…" : "Reconcile receipt"}
              </button>
            )}
          </>
        ) : (
          <EmptyState title="Select an invocation">
            Inspect its state, payment reference, and settled output.
          </EmptyState>
        )}
        {error && (
          <p className="ws-error" role="alert">
            {error}
          </p>
        )}
      </section>
    </div>
  );
};

export const SalesPanel = ({
  sales,
  enabled,
  onConnect,
}: {
  sales: Sale[];
  enabled: boolean;
  onConnect: () => void;
}) => (
  <section className="ws-panel">
    <div className="ws-panel-title">
      <h2>Creator sales</h2>
      <span className="ws-muted">Latest 100</span>
    </div>
    <p className="ws-muted">
      Completed transfers to your wallet. Dodo entries are labeled as test
      accounting.
    </p>
    {enabled && sales.length > 0 && (
      <div className="ws-history">
        {sales.map((sale) => (
          <div className="ws-history-row" key={sale.invocation_id}>
            <span>
              <strong>{shortId(sale.invocation_id)}</strong>
              <small>{new Date(sale.created_at * 1000).toLocaleString()}</small>
            </span>
            <span>
              {sale.network === "dodo:test"
                ? "Dodo · test accounting"
                : sale.network}
            </span>
            <span className="ws-numeric">{atomicMoney(sale.amount)} USDC</span>
            <code title={sale.transaction_hash}>
              {shortId(sale.transaction_hash)}
            </code>
          </div>
        ))}
      </div>
    )}
    {enabled && sales.length === 0 && (
      <EmptyState title="Room for your first sale.">
        Capabilities published by your agent will appear in Account settings.
        Completed sales will appear here.
      </EmptyState>
    )}
    {!enabled && (
      <EmptyState title="Connect a creator session">
        <button type="button" className="ws-button" onClick={onConnect}>
          Connect with sales access
        </button>
      </EmptyState>
    )}
  </section>
);

const Checkout = ({ id, token }: { id: string; token: string }) => {
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  return (
    <div>
      <button
        className="ws-button"
        type="button"
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          setError("");
          try {
            const checkout = await hiveway<{ checkout_url: string }>(
              `/v1/invocations/${id}/checkout`,
              token,
              { body: "{}", method: "POST" }
            );
            const url = new URL(checkout.checkout_url);
            if (url.protocol !== "https:") {
              setError("Invalid checkout URL");
              setBusy(false);
              return;
            }
            window.location.assign(url.href);
          } catch (error_) {
            setError(errorMessage(error_));
          }
          setBusy(false);
        }}
      >
        Open Dodo test checkout
      </button>
      <p>
        Test payments only. For x402, use the signed payment handoff below.
        Completion is verified by the backend; returning from checkout does not
        establish payment.
      </p>
      {error && <p role="alert">{error}</p>}
    </div>
  );
};

export const QuoteForm = ({
  tool,
  token,
  onQuoted,
  onConnect,
}: {
  tool: RegistryCapability;
  token: string;
  onQuoted: () => void;
  onConnect: () => void;
}) => {
  const [url, setUrl] = useState("");
  const [revision, setRevision] = useState("");
  const [jsonInput, setJsonInput] = useState("{}");
  const [freeReceipt, setFreeReceipt] = useState<Receipt | null>(null);
  const [quote, setQuote] = useState<Quote | null>(null);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(0);
  const request = useRef({ fingerprint: "", key: "" });
  let quoteLabel = token ? "Get a quote" : "Connect to get a quote";
  if (busy) {
    quoteLabel = "Reserving quote…";
  }
  useEffect(() => {
    if (!quote) {
      return;
    }
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [quote]);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!token) {
      onConnect();
      return;
    }
    setBusy(true);
    setQuote(null);
    setFreeReceipt(null);
    setNow(Date.now());
    setError("");
    try {
      if (tool.runtime !== "wasm") {
        validateRepository(url.trim(), revision.trim());
      }
      const body = JSON.stringify({
        capability_id: tool.id,
        input:
          tool.runtime === "wasm"
            ? (JSON.parse(jsonInput) as unknown)
            : { repository_url: url.trim(), revision: revision.trim() },
      });
      if (request.current.fingerprint !== body) {
        request.current = { fingerprint: body, key: crypto.randomUUID() };
      }
      const result = await hiveway<Quote>("/v1/quotes", token, {
        body,
        headers: { "Idempotency-Key": request.current.key },
        method: "POST",
      });
      if (result.payment.accepts.length === 0) {
        setFreeReceipt(
          await hiveway<Receipt>(`/v1/invocations/${result.id}`, token, {
            body: "{}",
            method: "POST",
          })
        );
      }
      setQuote(result);
      setCopied(false);
      setNow(Date.now());
      onQuoted();
    } catch (error_) {
      setError(errorMessage(error_));
    }
    setBusy(false);
  };
  return (
    <form
      className="ws-form"
      onSubmit={(e) => {
        void submit(e);
      }}
    >
      {tool.runtime === "wasm" ? (
        <label className="ws-field">
          JSON input
          <textarea
            aria-label="JSON input"
            value={jsonInput}
            onChange={(event) => {
              setJsonInput(event.target.value);
              setQuote(null);
              setFreeReceipt(null);
            }}
            rows={7}
            required
          />
        </label>
      ) : (
        <>
          <label className="ws-field">
            Public repository
            <input
              type="url"
              required
              placeholder="https://github.com/owner/repo"
              value={url}
              onChange={(e) => {
                setUrl(e.target.value);
                setQuote(null);
              }}
            />
          </label>
          <label className="ws-field">
            Commit SHA
            <input
              required
              placeholder="Full 40-character commit hash"
              pattern="[a-fA-F0-9]{40}"
              value={revision}
              onChange={(e) => {
                setRevision(e.target.value);
                setQuote(null);
              }}
            />
            <small>
              Immutable commit required. Branches and tags are not accepted.
            </small>
          </label>
        </>
      )}
      <button
        className="ws-button ws-primary"
        type={token ? "submit" : "button"}
        disabled={busy || Boolean(quote)}
        onClick={token ? undefined : onConnect}
      >
        {quoteLabel}
        <Icon name="arrow" size={15} />
      </button>
      <p className="ws-fine">
        A quote reserves session budget for 120 seconds. Payment needs a
        separate authorization.
      </p>
      {quote && (
        <div className="ws-quote">
          <strong>
            {Math.max(0, Math.ceil((quote.expires_at * 1000 - now) / 1000))}s
            remaining
          </strong>
          <code>{quote.id}</code>
          {freeReceipt ? (
            <>
              <Status state={freeReceipt.state} />
              <Json value={freeReceipt.result} />
            </>
          ) : (
            <>
              <Checkout id={quote.id} token={token} />
              <p className="ws-fine">No payment has been submitted here.</p>
            </>
          )}
          <details className="ws-disclosure">
            <summary>Quote data for your agent</summary>
            <Json
              value={{
                endpoint: `${apiOrigin}/v1/invocations/${quote.id}`,
                invocation_id: quote.id,
                payment: quote.payment,
              }}
            />
          </details>
          <button
            className="ws-button"
            type="button"
            onClick={async () => {
              try {
                await navigator.clipboard.writeText(
                  JSON.stringify(
                    {
                      endpoint: `${apiOrigin}/v1/invocations/${quote.id}`,
                      invocation_id: quote.id,
                      payment: quote.payment,
                    },
                    null,
                    2
                  )
                );
                setCopied(true);
              } catch {
                setError(
                  "Clipboard unavailable. Select the quote above to copy it."
                );
              }
            }}
          >
            {copied ? "Payment handoff copied" : "Copy payment handoff"}{" "}
            <Icon name={copied ? "check" : "copy"} size={14} />
          </button>
          {now >= quote.expires_at * 1000 && (
            <button
              className="ws-text-button"
              type="button"
              onClick={() => {
                setQuote(null);
                request.current = { fingerprint: "", key: "" };
              }}
            >
              Start a new quote
            </button>
          )}
        </div>
      )}
      {error && (
        <p role="alert" className="ws-error">
          {error}
        </p>
      )}
    </form>
  );
};

const SampleForm = ({
  tool,
  token,
}: {
  tool: RegistryCapability;
  token: string;
}) => {
  const [input, setInput] = useState(
    JSON.stringify(
      tool.runtime === "wasm"
        ? (tool.definition.sample_input ?? {})
        : {
            repository_url: "https://github.com/octocat/Hello-World",
            revision: "7fd1a60b01f91b314f59955a4e4d4c63fbd0fa49",
          },
      null,
      2
    )
  );
  const [result, setResult] = useState<unknown>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  return (
    <form
      className="ws-form"
      onSubmit={async (event) => {
        event.preventDefault();
        setBusy(true);
        setError("");
        setResult(null);
        try {
          const data = await hiveway<{ result: unknown }>(
            `/v1/capabilities/${tool.id}/test`,
            token,
            {
              body: JSON.stringify({ input: JSON.parse(input) as unknown }),
              method: "POST",
            }
          );
          setResult(data.result);
        } catch (error_) {
          setError(errorMessage(error_));
        }
        setBusy(false);
      }}
    >
      <p>
        Run editable input against the actual runtime. Paid sample tests require
        the creator’s wallet session.
      </p>
      {tool.runtime === "wasm" &&
        typeof tool.definition.artifact_id === "string" && (
          <button
            className="ws-button"
            type="button"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              setError("");
              try {
                const artifact = await hiveway<{ input_schema: unknown }>(
                  `/v1/artifacts/${String(tool.definition.artifact_id)}`,
                  token
                );
                setInput(
                  JSON.stringify(sampleInput(artifact.input_schema), null, 2)
                );
              } catch (error_) {
                setError(errorMessage(error_));
              }
              setBusy(false);
            }}
          >
            Load sample from artifact schema
          </button>
        )}
      <label className="ws-field">
        Sample JSON
        <textarea
          rows={10}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          required
        />
      </label>
      <button className="ws-button ws-primary" disabled={busy} type="submit">
        {busy ? "Running…" : "Run test"}
      </button>
      {error && <p role="alert">{error}</p>}
      {result !== null && <Json value={result} />}
    </form>
  );
};

export const CapabilityActionDialog = ({
  mode,
  tool,
  token,
  onConnect,
  onRefresh,
  onClose,
}: {
  mode: "sample" | "invoke";
  tool: RegistryCapability;
  token: string;
  onConnect: () => void;
  onRefresh: () => void;
  onClose: () => void;
}) => {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  return (
    <dialog
      ref={dialog}
      className="ws-dialog ws-capability-dialog"
      onClose={onClose}
      aria-labelledby="capability-action-title"
    >
      <div className="ws-dialog-heading">
        <div>
          <h2 id="capability-action-title">
            {mode === "sample" ? "Test with sample input" : "Invoke in agent"}
          </h2>
          <p className="ws-muted">{tool.name}</p>
        </div>
        <button
          className="ws-icon-button"
          type="button"
          onClick={onClose}
          aria-label="Close capability action"
        >
          <Icon name="close" size={18} />
        </button>
      </div>
      {mode === "sample" ? (
        <SampleForm tool={tool} token={token} />
      ) : (
        <QuoteForm
          tool={tool}
          token={token}
          onQuoted={onRefresh}
          onConnect={() => {
            onClose();
            onConnect();
          }}
        />
      )}
    </dialog>
  );
};
