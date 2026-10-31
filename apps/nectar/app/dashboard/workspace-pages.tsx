"use client";

import { useEffect, useRef, useState } from "react";
import type { SubmitEvent } from "react";

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
  <div className="ws-empty flex min-h-[220px] flex-col items-center justify-center gap-[13px] px-5 py-[30px] text-center text-[#686f60] [&_.ws-button]:mx-auto [&_.ws-button]:mt-[15px] [&_.ws-button]:mb-0 [&_.ws-button]:flex [&_.ws-text-button]:mx-auto [&_.ws-text-button]:mt-[15px] [&_.ws-text-button]:mb-0 [&_.ws-text-button]:flex [&_h3]:text-[16px] [&_h3]:font-medium [&_h3]:tracking-[-0.04em] [&_h3]:text-[#686f60] [&_p]:max-w-[300px] [&_p]:text-[11px] [&_p]:leading-[1.9]">
    <Icon name="layers" size={25} />
    <h3 className="font-display">{title}</h3>
    <p>{children}</p>
  </div>
);
export const Json = ({ value }: { value: unknown }) => (
  <pre className="ws-json mx-0 my-[13px] max-h-[300px] overflow-auto rounded-[8px] bg-[#f6f8f1] p-[13px] text-[10px] leading-[1.85] [overflow-wrap:anywhere] whitespace-pre-wrap text-[#686f60] [border:1px_solid_#e4e8dc]">
    {JSON.stringify(value, null, 2)}
  </pre>
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
    <span
      className={`ws-status inline-flex shrink-0 items-center gap-[5px] rounded-[5px] pt-1 pr-1.5 pb-1 pl-1.5 text-[10px] text-[color:#686f60] capitalize [background:#f6f1e5] [&.is-error]:text-[color:#686f60] [&.is-error]:[background:#fcf0e8] [&.is-success]:text-[color:#686f60] [&.is-success]:[background:#eff5e9] [&>span]:h-1 [&>span]:w-1 [&>span]:rounded-full [&>span]:[background:currentColor] ${appearance}`}
    >
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
  const connect = async (event?: SubmitEvent<HTMLFormElement>) => {
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
      className="ws-dialog m-auto max-h-[calc(100dvh_-_40px)] w-[min(470px,_calc(100%_-_28px))] overflow-y-auto rounded-[20px] bg-[#fcfdf9] p-[26px] text-[#686f60] shadow-[0_30px_90px_#202d1430] [border:1px_solid_#e0e6d3] max-[760px]:p-[22px] [&_.ws-disclosure_form]:grid [&_.ws-disclosure_form]:gap-[15px] [&_.ws-disclosure_form]:px-0 [&_.ws-disclosure_form]:py-3 [&_.ws-fine]:mt-[18px] [&_.ws-fine]:text-[10px] [&_h2]:mt-[9px] [&_h2]:text-[24px] [&_h2]:tracking-[-0.055em] [&_h2]:text-[#4c603d] max-[760px]:[&_h2]:text-[22px] [&::backdrop]:bg-[#202b1a55] [&::backdrop]:[backdrop-filter:blur(4px)] [&>.ws-button]:w-full [&>p]:text-[12px] [&>p]:leading-[1.9]"
      onClose={onClose}
      aria-labelledby="session-title"
    >
      <div className="ws-dialog-heading mb-[18px] flex items-start justify-between gap-3">
        <div>
          <span className="ws-overline text-[10px] font-bold tracking-[0.13em] text-[#686f60] uppercase max-[760px]:text-[10px] max-[760px]:tracking-[0.1em]">
            YOUR WORKSPACE
          </span>
          <h2 id="session-title" className="font-display">
            Connect to the hive.
          </h2>
        </div>
        <button
          className="ws-icon-button inline-flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:bg-[#edeee7]"
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
      <div className="ws-note mx-0 my-4 block rounded-[9px] bg-[#f3f6eb] p-3 text-[11px] leading-[1.9] text-[#686f60] [border:1px_solid_#e0e7d3] [&_.ws-text-button]:flex">
        1 hour session · 1 USDC per call · 10 USDC total authorization. Signing
        in does not authorize a payment.
      </div>
      <label className="ws-checkbox mx-0 my-3.5 flex items-center gap-[9px] text-[11px] leading-[1.8] text-[#686f60] [&_input]:[accent-color:#6f845a] [&_small]:block [&_small]:text-[10px] [&_small]:text-[#686f60]">
        <input
          type="checkbox"
          checked={creator}
          onChange={(e) => setCreator(e.target.checked)}
        />
        Include creator sales access
      </label>
      <button
        type="button"
        className="ws-button ws-primary inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#40473a] [&:hover]:bg-[#f4f5ee]"
        disabled={busy}
        onClick={() => {
          void connect();
        }}
      >
        <Icon name="wallet" size={16} />
        {busy ? "Connecting…" : "Connect wallet"}
      </button>
      <details className="ws-disclosure text-[10px] text-[#686f60] [border-bottom:1px_solid_var(--ws-line)] [&_p]:pb-[13px] [&_p]:leading-[1.9] [&_p]:text-[#686f60] [&_summary]:px-0 [&_summary]:py-3">
        <summary className="cursor-pointer">
          Use an existing scoped credential
        </summary>
        <form
          onSubmit={(event) => {
            void connect(event);
          }}
        >
          <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
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
            className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
            disabled={busy || !token.trim()}
            type="submit"
          >
            Connect credential
          </button>
        </form>
      </details>
      <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
        Credentials stay in memory and clear when this page reloads.
      </p>
      {error && (
        <p
          className="ws-error mt-[13px] text-[11px] leading-[1.8] text-[#a45541]"
          role="alert"
        >
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
  onLoadMore,
  hasMore,
  loadingMore,
}: {
  token: string;
  activity: Activity | null;
  catalog: RegistryCapability[];
  onRefresh: () => void;
  onLoadMore: () => void;
  hasMore: boolean;
  loadingMore: boolean;
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
    <div className="ws-page-grid grid grid-cols-[minmax(0,_1.3fr)_minmax(280px,_1fr)] items-start gap-[22px] max-[1100px]:grid-cols-[1fr]">
      <section className="ws-panel min-w-0 rounded-[14px] bg-white p-6 [border:1px_solid_var(--ws-line)] max-[760px]:p-[19px]">
        <div className="ws-panel-title mb-[15px] flex items-center gap-[9px] text-[#686f60] [&>h2]:[flex:1] [&>h2]:text-[16px] [&>h2]:[font-weight:650] [&>h2]:tracking-[-0.04em] [&>h2]:text-[#4a573e]">
          <h2 className="font-display">Invocation history</h2>
          <button
            className="ws-text-button inline-flex cursor-pointer items-center gap-[5px] px-0 py-1.5 text-[11px] font-semibold text-[#5f7651] [background:none] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:text-[#273d1c] [&:hover]:[text-decoration:underline]"
            type="button"
            onClick={onRefresh}
          >
            Refresh <Icon name="activity" size={14} />
          </button>
        </div>
        <p className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
          Recent account activity. Updates every 3 seconds while this page is
          open.
        </p>
        {activity?.counts && (
          <p className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
            {activity.counts.total} invocations · {activity.counts.succeeded}{" "}
            succeeded · {activity.counts.failed} failed
          </p>
        )}
        {activity?.invocations.length ? (
          <div className="ws-history mt-5">
            {activity.invocations.map((item) => (
              <button
                key={item.id}
                type="button"
                className="ws-history-row flex w-full cursor-pointer items-center gap-4 px-0 py-[15px] text-left text-[10px] text-[#686f60] [background:none] [border-top:1px_solid_var(--ws-line)] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] max-[760px]:flex-wrap max-[760px]:gap-2.5 [&_small]:mt-[5px] [&_small]:block [&_small]:text-[10px] [&_small]:text-[#686f60] [&_strong]:block [&_strong]:overflow-hidden [&_strong]:text-[11px] [&_strong]:text-ellipsis [&_strong]:text-[#686f60] max-[760px]:[&>.ws-numeric]:ml-auto max-[760px]:[&>code]:w-full max-[760px]:[&>code]:[overflow-wrap:anywhere] [&>span:first-child]:min-w-0 [&>span:first-child]:[flex:1] max-[760px]:[&>span:first-child]:[flex-basis:45%]"
                onClick={() => {
                  void load(item.id);
                }}
              >
                <span>
                  <strong>
                    {item.capability_name ??
                      catalog.find((tool) => tool.id === item.capability_id)
                        ?.name ??
                      shortId(item.capability_id)}
                  </strong>
                  <small>
                    {new Date(item.created_at * 1000).toLocaleString()} ·{" "}
                    {item.payment_method ?? "Unpaid"}
                  </small>
                  <small>
                    {item.agent_name ?? "Unknown agent"} ·{" "}
                    {item.client_context?.harness ?? "Unknown harness"} ·{" "}
                    {item.client_context?.model ?? "Model not reported"}
                  </small>
                </span>
                <Status state={item.state} />
                <span className="ws-numeric text-right [font-variant-numeric:tabular-nums]">
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
        {hasMore && (
          <button
            className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
            type="button"
            onClick={onLoadMore}
            disabled={loadingMore}
          >
            {loadingMore ? "Loading…" : "Load older activity"}
          </button>
        )}
      </section>
      <section className="ws-panel min-w-0 rounded-[14px] bg-white p-6 [border:1px_solid_var(--ws-line)] max-[760px]:p-[19px]">
        <div className="ws-panel-title mb-[15px] flex items-center gap-[9px] text-[#686f60] [&>h2]:[flex:1] [&>h2]:text-[16px] [&>h2]:[font-weight:650] [&>h2]:tracking-[-0.04em] [&>h2]:text-[#4a573e]">
          <h2 className="font-display">Receipt &amp; result</h2>
        </div>
        {receipt ? (
          <>
            <div className="ws-receipt-meta mb-[15px] flex flex-col items-start gap-3.5 [&_code]:text-[10px] [&_code]:[overflow-wrap:anywhere] [&_code]:text-[#686f60]">
              <Status state={receipt.state} />
              <code>{receipt.id}</code>
            </div>
            {receipt.error_code && (
              <p className="ws-error mt-[13px] text-[11px] leading-[1.8] text-[#a45541]">
                {receipt.error_code.replaceAll("_", " ")}
              </p>
            )}
            {receipt.payment_issue && (
              <p className="ws-note mx-0 my-4 block rounded-[9px] bg-[#f3f6eb] p-3 text-[11px] leading-[1.9] text-[#686f60] [border:1px_solid_#e0e7d3] [&_.ws-text-button]:flex">
                Payment: {receipt.payment_issue.replaceAll("_", " ")}
              </p>
            )}
            {receipt.state === "settlement_unknown" ||
            receipt.state === "settling" ? (
              <p className="ws-note mx-0 my-4 block rounded-[9px] bg-[#f3f6eb] p-3 text-[11px] leading-[1.9] text-[#686f60] [border:1px_solid_#e0e7d3] [&_.ws-text-button]:flex">
                Payment finality is pending. Keep this invocation; do not pay
                again.
              </p>
            ) : null}
            {receipt.state === "settled" ? (
              <>
                <Json value={receipt.result} />
                {receipt.settlement && (
                  <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
                    {receipt.settlement.network}
                    <br />
                    <code>{receipt.settlement.transaction}</code>
                  </p>
                )}
              </>
            ) : (
              <button
                type="button"
                className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
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
          <p
            className="ws-error mt-[13px] text-[11px] leading-[1.8] text-[#a45541]"
            role="alert"
          >
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
  <section className="ws-panel min-w-0 rounded-[14px] bg-white p-6 [border:1px_solid_var(--ws-line)] max-[760px]:p-[19px]">
    <div className="ws-panel-title mb-[15px] flex items-center gap-[9px] text-[#686f60] [&>h2]:[flex:1] [&>h2]:text-[16px] [&>h2]:[font-weight:650] [&>h2]:tracking-[-0.04em] [&>h2]:text-[#4a573e]">
      <h2 className="font-display">Creator sales</h2>
      <span className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
        Latest 100
      </span>
    </div>
    <p className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
      Completed transfers to your wallet. Dodo entries are labeled as test
      accounting.
    </p>
    {enabled && sales.length > 0 && (
      <div className="ws-history mt-5">
        {sales.map((sale) => (
          <div
            className="ws-history-row flex w-full items-center gap-4 px-0 py-[15px] text-left text-[10px] text-[#686f60] [background:none] [border-top:1px_solid_var(--ws-line)] [border:0] max-[760px]:flex-wrap max-[760px]:gap-2.5 [&_small]:mt-[5px] [&_small]:block [&_small]:text-[10px] [&_small]:text-[#686f60] [&_strong]:block [&_strong]:overflow-hidden [&_strong]:text-[11px] [&_strong]:text-ellipsis [&_strong]:text-[#686f60] max-[760px]:[&>.ws-numeric]:ml-auto max-[760px]:[&>code]:w-full max-[760px]:[&>code]:[overflow-wrap:anywhere] [&>span:first-child]:min-w-0 [&>span:first-child]:[flex:1] max-[760px]:[&>span:first-child]:[flex-basis:45%]"
            key={sale.invocation_id}
          >
            <span>
              <strong>{shortId(sale.invocation_id)}</strong>
              <small>{new Date(sale.created_at * 1000).toLocaleString()}</small>
            </span>
            <span>
              {sale.network === "dodo:test"
                ? "Dodo · test accounting"
                : sale.network}
            </span>
            <span className="ws-numeric text-right [font-variant-numeric:tabular-nums]">
              {atomicMoney(sale.amount)} USDC
            </span>
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
        <button
          type="button"
          className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
          onClick={onConnect}
        >
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
        className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
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
  const [jsonInput, setJsonInput] = useState(
    tool.runtime === "repository_scaffold"
      ? JSON.stringify(
          { project_name: "my-app", repository_url: "", revision: "" },
          null,
          2
        )
      : "{}"
  );
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
  const submit = async (e: SubmitEvent<HTMLFormElement>) => {
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
      if (tool.runtime !== "wasm" && tool.runtime !== "repository_scaffold") {
        validateRepository(url.trim(), revision.trim());
      }
      const body = JSON.stringify({
        capability_id: tool.id,
        input:
          tool.runtime === "wasm" || tool.runtime === "repository_scaffold"
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
      className="ws-form mt-[18px] flex flex-col items-stretch gap-[18px] max-[760px]:[&>.ws-button]:min-h-[44px]"
      onSubmit={(e) => {
        void submit(e);
      }}
    >
      {tool.runtime === "wasm" || tool.runtime === "repository_scaffold" ? (
        <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
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
          <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
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
          <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
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
        className="ws-button ws-primary inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#40473a] [&:hover]:bg-[#f4f5ee]"
        type={token ? "submit" : "button"}
        disabled={busy || Boolean(quote)}
        onClick={token ? undefined : onConnect}
      >
        {quoteLabel}
        <Icon name="arrow" size={15} />
      </button>
      <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
        A quote reserves session budget for 120 seconds. Payment needs a
        separate authorization.
      </p>
      {quote && (
        <div className="ws-quote min-w-0 pt-[13px] [border-top:1px_solid_var(--ws-line)] [&>code]:text-[10px] [&>code]:[overflow-wrap:anywhere] [&>strong]:mb-2 [&>strong]:block [&>strong]:text-[11px] [&>strong]:text-[#686f60]">
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
              <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
                No payment has been submitted here.
              </p>
            </>
          )}
          <details className="ws-disclosure text-[10px] text-[#686f60] [border-bottom:1px_solid_var(--ws-line)] [&_p]:pb-[13px] [&_p]:leading-[1.9] [&_p]:text-[#686f60] [&_summary]:px-0 [&_summary]:py-3">
            <summary className="cursor-pointer">
              Quote data for your agent
            </summary>
            <Json
              value={{
                endpoint: `${apiOrigin}/v1/invocations/${quote.id}`,
                invocation_id: quote.id,
                payment: quote.payment,
              }}
            />
          </details>
          <button
            className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
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
              className="ws-text-button inline-flex cursor-pointer items-center gap-[5px] px-0 py-1.5 text-[11px] font-semibold text-[#5f7651] [background:none] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:text-[#273d1c] [&:hover]:[text-decoration:underline]"
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
        <p
          role="alert"
          className="ws-error mt-[13px] text-[11px] leading-[1.8] text-[#a45541]"
        >
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
            ...(tool.runtime === "repository_scaffold"
              ? { project_name: "my-project" }
              : {}),
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
      className="ws-form mt-[18px] flex flex-col items-stretch gap-[18px] max-[760px]:[&>.ws-button]:min-h-[44px]"
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
      {(tool.runtime === "wasm" || tool.runtime === "repository_scaffold") &&
        typeof tool.definition.artifact_id === "string" && (
          <button
            className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
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
      <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
        Sample JSON
        <textarea
          rows={10}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          required
        />
      </label>
      <button
        className="ws-button ws-primary inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#40473a] [&:hover]:bg-[#f4f5ee]"
        disabled={busy}
        type="submit"
      >
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
      className="ws-dialog ws-capability-dialog m-auto max-h-[calc(100dvh_-_40px)] w-[min(470px,_calc(100%_-_28px))] overflow-y-auto rounded-[20px] bg-[#fcfdf9] p-[26px] text-[#686f60] shadow-[0_30px_90px_#202d1430] [border:1px_solid_#e0e6d3] max-[760px]:p-[22px] [&_.ws-dialog-heading_.ws-muted]:mt-[9px] [&_.ws-dialog-heading_.ws-muted]:text-[12px] [&_.ws-dialog-heading_h2]:mt-0 [&_.ws-dialog-heading_h2]:text-[22px] [&_.ws-disclosure_form]:grid [&_.ws-disclosure_form]:gap-[15px] [&_.ws-disclosure_form]:px-0 [&_.ws-disclosure_form]:py-3 [&_.ws-fine]:mt-[18px] [&_.ws-fine]:text-[10px] [&_h2]:mt-[9px] [&_h2]:text-[24px] [&_h2]:tracking-[-0.055em] [&_h2]:text-[#4c603d] max-[760px]:[&_h2]:text-[22px] [&::backdrop]:bg-[#202b1a55] [&::backdrop]:[backdrop-filter:blur(4px)] [&>.ws-button]:w-full [&>p]:text-[12px] [&>p]:leading-[1.9]"
      onClose={onClose}
      aria-labelledby="capability-action-title"
    >
      <div className="ws-dialog-heading mb-[18px] flex items-start justify-between gap-3">
        <div>
          <h2 id="capability-action-title" className="font-display">
            {mode === "sample" ? "Test with sample input" : "Invoke in agent"}
          </h2>
          <p className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
            {tool.name}
          </p>
        </div>
        <button
          className="ws-icon-button inline-flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:bg-[#edeee7]"
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
