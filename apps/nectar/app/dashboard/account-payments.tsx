"use client";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import type { SubmitEvent } from "react";

import { Icon } from "../../components/icon";
import type { WalletBalance } from "../../lib/account";
import { sendUsdc, networkLabel, parseTransferAmount } from "../../lib/account";
import { atomicMoney, errorMessage, hiveway, shortId } from "../../lib/hiveway";

export const SidebarBalance = ({
  token,
  credentialId,
  onOpen,
}: {
  token: string;
  credentialId: string | undefined;
  onOpen: () => void;
}) => {
  const balance = useQuery({
    enabled: Boolean(token && credentialId),
    queryFn: ({ signal }) =>
      hiveway<WalletBalance>("/v1/account/balance", token, { signal }),
    queryKey: ["hive", credentialId, "wallet-balance"],
  });
  return (
    <button
      className="ws-sidebar-balance [&>strong>small]:font-body block w-full cursor-pointer rounded-[10px] bg-[#fafbf7] px-3 py-[13px] text-left [border:1px_solid_var(--ws-line)] disabled:cursor-not-allowed disabled:opacity-[0.45] [&_.ws-balance-track]:h-[5px] [&:hover]:[border-color:#c7d0bb] [&:hover]:bg-[#f5f7ef] [&>.ws-balance-health]:mt-3 [&>.ws-balance-health]:block [&>strong]:mt-2.5 [&>strong]:flex [&>strong]:items-baseline [&>strong]:gap-1.5 [&>strong]:font-mono [&>strong]:text-[20px] [&>strong]:font-medium [&>strong]:tracking-[-0.04em] [&>strong]:text-[#303a27] [&>strong>small]:text-[10px] [&>strong>small]:tracking-[0] [&>strong>small]:text-[#727c67]"
      type="button"
      onClick={onOpen}
      aria-label="Open agent balance in Account settings"
    >
      <span className="ws-sidebar-balance-label flex items-center justify-between text-[11px] text-[#727c67]">
        Agent balance <Icon name="wallet" size={14} />
      </span>
      <strong>
        {typeof balance.data?.amount === "string" ? (
          <>
            {atomicMoney(balance.data.amount)} <small>USDC</small>
          </>
        ) : (
          "Free access"
        )}
      </strong>
      <span className="ws-sidebar-balance-link mt-2.5 flex items-center justify-between text-[10px] text-[#727c67]">
        {balance.data?.wallet ? "Manage balance" : "Wallet optional"}
        <Icon name="chevron" size={12} />
      </span>
    </button>
  );
};
type PaymentAction = "topup" | "send" | "receive";
const titles: Record<PaymentAction, string> = {
  receive: "Receive funds",
  send: "Send USDC",
  topup: "Fund your wallet",
};
const AccountPaymentDialog = ({
  action,
  balance,
  onClose,
  onRefresh,
}: {
  action: PaymentAction;
  balance: WalletBalance;
  onClose: () => void;
  onRefresh: () => void;
}) => {
  const ref = useRef<HTMLDialogElement>(null);
  const [amount, setAmount] = useState("");
  const [recipient, setRecipient] = useState("");
  const [message, setMessage] = useState("");
  const [hash, setHash] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  const submit = async (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setMessage("");
    try {
      setHash(
        await sendUsdc(balance, recipient.trim(), parseTransferAmount(amount))
      );
      setMessage(
        "Transaction submitted. Confirm it in your wallet, then refresh the balance after confirmation."
      );
    } catch (error) {
      setMessage(errorMessage(error));
    }
    setBusy(false);
  };
  return (
    <dialog
      ref={ref}
      className="ws-dialog ws-funding-dialog m-auto max-h-[calc(100dvh_-_40px)] w-[min(470px,_calc(100%_-_28px))] overflow-y-auto rounded-[20px] bg-[#fcfdf9] p-[26px] text-[#686f60] shadow-[0_30px_90px_#202d1430] [border:1px_solid_#e0e6d3] max-[760px]:p-[22px] [&_.ws-disclosure_form]:grid [&_.ws-disclosure_form]:gap-[15px] [&_.ws-disclosure_form]:px-0 [&_.ws-disclosure_form]:py-3 [&_.ws-fine]:mt-[18px] [&_.ws-fine]:text-[10px] [&_form>.ws-button]:mt-3 [&_form>.ws-button]:w-full [&_h2]:mt-[9px] [&_h2]:text-[24px] [&_h2]:tracking-[-0.055em] [&_h2]:text-[#4c603d] max-[760px]:[&_h2]:text-[22px] [&::backdrop]:bg-[#202b1a55] [&::backdrop]:[backdrop-filter:blur(4px)] [&>.ws-button]:w-full [&>p]:text-[12px] [&>p]:leading-[1.9]"
      onClose={onClose}
      aria-labelledby="funding-title"
    >
      <div className="ws-dialog-heading mb-[18px] flex items-start justify-between gap-3">
        <div>
          <span className="ws-overline text-[10px] font-bold tracking-[0.13em] text-[#686f60] uppercase max-[760px]:text-[10px] max-[760px]:tracking-[0.1em]">
            AGENT BALANCE
          </span>
          <h2 id="funding-title" className="font-display">
            {titles[action]}
          </h2>
        </div>
        <button
          type="button"
          className="ws-icon-button inline-flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-[8px] bg-transparent text-[#686f60] [border:0] disabled:cursor-not-allowed disabled:opacity-[0.45] [&:hover]:bg-[#edeee7]"
          onClick={onClose}
          aria-label="Close payment dialog"
        >
          <Icon name="close" size={18} />
        </button>
      </div>
      <div className="ws-payment-provider mx-0 my-[22px] flex items-center gap-3 rounded-[12px] bg-[#fafbf7] p-[15px] [border:1px_solid_var(--ws-line)] [&_.ws-badge]:text-[10px] [&_small]:mt-[5px] [&_small]:block [&_small]:text-[10px] [&_small]:text-[color:var(--ws-muted)] [&_strong]:block [&_strong]:text-[13px] [&>div]:[flex:1]">
        <span className="ws-payment-provider-icon grid h-[38px] w-[38px] shrink-0 [place-items:center] rounded-[10px] bg-[#eff2e7] text-[#657654]">
          <Icon name="wallet" size={21} />
        </span>
        <div>
          <strong>Connected wallet</strong>
          <small>{networkLabel(balance.chain_id)}</small>
        </div>
      </div>
      {action === "send" ? (
        <form
          className="ws-payment-fields [&>.ws-button]:mt-3 [&>.ws-button]:w-full"
          onSubmit={(event) => {
            void submit(event);
          }}
        >
          <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
            Recipient wallet
            <input
              required
              value={recipient}
              onChange={(event) => setRecipient(event.target.value)}
              placeholder="0x…"
              autoComplete="off"
            />
          </label>
          <label className="ws-field flex flex-col gap-[7px] text-[10px] [font-weight:650] text-[#667655] [&_input]:w-full [&_input]:min-w-0 [&_input]:rounded-[8px] [&_input]:bg-white [&_input]:px-[11px] [&_input]:py-2.5 [&_input]:text-[12px] [&_input]:font-normal [&_input]:text-[#4e5c40] [&_input]:[border:1px_solid_#dfe4d5] max-[760px]:[&_input]:text-[16px] [&_input::placeholder]:text-[#686f60] [&_select]:w-full [&_select]:min-w-0 [&_select]:rounded-[8px] [&_select]:bg-white [&_select]:px-[11px] [&_select]:py-2.5 [&_select]:text-[12px] [&_select]:font-normal [&_select]:text-[#4e5c40] [&_select]:[border:1px_solid_#dfe4d5] max-[760px]:[&_select]:text-[16px] [&_small]:text-[10px] [&_small]:leading-[1.7] [&_small]:font-normal [&_small]:text-[#686f60] [&_textarea]:w-full [&_textarea]:min-w-0 [&_textarea]:[resize:vertical] [&_textarea]:rounded-[8px] [&_textarea]:bg-white [&_textarea]:px-[11px] [&_textarea]:py-2.5 [&_textarea]:text-[12px] [&_textarea]:font-normal [&_textarea]:text-[#4e5c40] [&_textarea]:[border:1px_solid_#dfe4d5] max-[760px]:[&_textarea]:text-[16px] [&_textarea::placeholder]:text-[#686f60]">
            Amount · USDC
            <input
              required
              inputMode="decimal"
              value={amount}
              onChange={(event) => setAmount(event.target.value)}
            />
          </label>
          <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
            Your wallet will show the recipient, token amount, and network fee
            before you approve.
          </p>
          <button
            type="submit"
            className="ws-button ws-primary inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#40473a] [&:hover]:bg-[#f4f5ee]"
            disabled={busy || Boolean(hash)}
          >
            {busy ? "Waiting for wallet…" : "Review in wallet"}
            <Icon name="arrow" size={14} />
          </button>
        </form>
      ) : (
        <div className="ws-payment-fields [&>.ws-button]:mt-3 [&>.ws-button]:w-full">
          <p className="ws-muted text-[11px] leading-[1.85] text-[#686f60]">
            Send USDC on {networkLabel(balance.chain_id)} to this wallet
            address. Funds remain in your wallet and can be used for capability
            payments.
          </p>
          <code className="ws-receive-address block rounded-[12px] bg-[#fafaf9] p-3 text-[12px] leading-[1.8] [overflow-wrap:anywhere] [border:1px_solid_#e7e5e4]">
            {balance.wallet}
          </code>
          <button
            className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
            type="button"
            onClick={async () => {
              try {
                await navigator.clipboard.writeText(balance.wallet ?? "");
                setMessage("Wallet address copied.");
              } catch (error) {
                setMessage(errorMessage(error));
              }
            }}
          >
            <Icon name="copy" size={14} /> Copy wallet address
          </button>
          {balance.chain_id === 84_532 && (
            <a
              className="ws-button ws-primary inline-flex min-h-[38px] items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#40473a] [&:hover]:bg-[#f4f5ee]"
              href="https://faucet.circle.com/"
              target="_blank"
              rel="noreferrer"
            >
              Get free Base Sepolia test USDC <Icon name="external" size={14} />
            </a>
          )}
          <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
            Dodo checkouts pay for individual invocations. They do not credit a
            wallet balance.
          </p>
        </div>
      )}
      {hash && (
        <>
          <code className="ws-receive-address block rounded-[12px] bg-[#fafaf9] p-3 text-[12px] leading-[1.8] [overflow-wrap:anywhere] [border:1px_solid_#e7e5e4]">
            {hash}
          </code>
          <button
            className="ws-button inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#f4f5ee]"
            type="button"
            onClick={onRefresh}
          >
            Refresh balance
          </button>
        </>
      )}
      {message && (
        <output className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
          {message}
        </output>
      )}
    </dialog>
  );
};

const PaymentConnection = ({
  balance,
  token,
}: {
  balance: WalletBalance | null;
  token: string;
}) => {
  const earnings = useQuery({
    enabled: Boolean(balance?.wallet),
    queryFn: ({ signal }) =>
      hiveway<{
        lifetime_amount: number;
        settled_runs: number;
        refunded_amount: number;
      }>("/v1/earnings/summary", token, { signal }),
    queryKey: ["hive", balance?.wallet, "balance-earnings"],
  });
  let checkout = "—";
  if (balance) {
    checkout = balance.dodo_test_checkout ? "Configured" : "Not configured";
  }
  return (
    <aside className="ws-account-secondary min-w-0 p-[var(--account-pad)] [border-left:1px_solid_var(--ws-line)] max-[900px]:[border-width:1px_0_0]">
      <div className="ws-account-section-title [&_h2>span]:font-body mb-[18px] flex items-center justify-between gap-3.5 max-[760px]:items-start max-[760px]:gap-2.5 [&_h2]:text-[16px] [&_h2]:[font-weight:650] [&_h2]:tracking-[-0.035em] [&_h2]:text-[#303a27] [&_h2>span]:ml-[5px] [&_h2>span]:text-[12px] [&_h2>span]:font-normal [&_h2>span]:text-[color:var(--ws-muted)] [&_p]:mt-1.5 [&_p]:text-[11px] [&_p]:leading-[1.7] [&_p]:text-[#727c67] [&>.ws-badge]:text-[10px] [&>.ws-badge]:whitespace-nowrap max-[760px]:[&>.ws-badge]:max-w-[115px] max-[760px]:[&>.ws-badge]:[white-space:normal]">
        <div>
          <h2 className="font-display">Payment connection</h2>
          <p>Funding and incoming payments.</p>
        </div>
      </div>
      <div className="ws-account-provider mx-0 mt-0 mb-[18px] flex items-center gap-2.5 [&_small]:mt-1 [&_small]:block [&_small]:text-[11px] [&_small]:text-[color:var(--ws-muted)] [&_strong]:block [&_strong]:text-[13px] [&_strong]:font-semibold [&_strong]:text-[#303a27]">
        <span className="ws-payment-provider-icon grid h-[38px] w-[38px] shrink-0 [place-items:center] rounded-[10px] bg-[#eff2e7] text-[#657654]">
          <Icon name="wallet" size={20} />
        </span>
        <div>
          <strong>
            {balance?.wallet ? shortId(balance.wallet) : "No wallet connected"}
          </strong>
          <small>
            {balance?.wallet
              ? networkLabel(balance.chain_id)
              : "Optional for paid capabilities"}
          </small>
        </div>
      </div>
      <dl className="ws-account-payment-facts m-0 [&_dd]:m-0 [&_dd]:text-right [&_dd]:text-[color:var(--ws-muted)] [&_dt]:text-[#626e56] [&>div]:flex [&>div]:items-center [&>div]:justify-between [&>div]:gap-3.5 [&>div]:px-0 [&>div]:py-3 [&>div]:text-[11px] [&>div]:[border-top:1px_solid_var(--ws-line)]">
        <div>
          <dt>Incoming payments</dt>
          <dd>Direct to wallet</dd>
        </div>
        <div>
          <dt>Creator earnings</dt>
          <dd>
            {earnings.data
              ? `${atomicMoney(String(earnings.data.lifetime_amount))} USDC`
              : "—"}
          </dd>
        </div>
        <div>
          <dt>Settled sales</dt>
          <dd>{earnings.data?.settled_runs ?? "—"}</dd>
        </div>
        <div>
          <dt>Dodo test checkout</dt>
          <dd>{checkout}</dd>
        </div>
      </dl>
      {earnings.error && (
        <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
          {errorMessage(earnings.error)}
        </p>
      )}
      <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
        Earnings are settlement history, including Dodo test accounting. They
        are not a withdrawable Hive balance.
      </p>
    </aside>
  );
};

export const AccountBalance = ({
  balance,
  error,
  loading,
  onRefresh,
  onConnect,
  token,
}: {
  balance: WalletBalance | null;
  error: string;
  loading: boolean;
  onRefresh: () => void;
  onConnect: () => void;
  token: string;
}) => {
  const [action, setAction] = useState<PaymentAction | null>(null);
  const connected = Boolean(balance?.wallet);
  const amount = balance?.amount;
  let status = connected
    ? "On-chain balance"
    : "Free capabilities are ready to use";
  if (loading) {
    status = "Checking wallet…";
  }
  return (
    <div className="ws-account-split grid grid-cols-[minmax(0,_1fr)_330px] items-stretch gap-0 max-[1100px]:grid-cols-[minmax(0,_1fr)_290px] max-[900px]:grid-cols-[minmax(0,_1fr)] min-[1700px]:grid-cols-[minmax(0,_1fr)_390px]">
      <section
        className="ws-account-balance mb-7 rounded-[13px] bg-white p-[22px] [border-bottom:1px_solid_var(--ws-line)] [border:1px_solid_var(--ws-line)] max-[760px]:p-[17px]"
        aria-labelledby="balance-heading"
      >
        <div className="ws-account-section-title [&_h2>span]:font-body mb-[18px] flex items-center justify-between gap-3.5 max-[760px]:items-start max-[760px]:gap-2.5 [&_h2]:text-[16px] [&_h2]:[font-weight:650] [&_h2]:tracking-[-0.035em] [&_h2]:text-[#303a27] [&_h2>span]:ml-[5px] [&_h2>span]:text-[12px] [&_h2>span]:font-normal [&_h2>span]:text-[color:var(--ws-muted)] [&_p]:mt-1.5 [&_p]:text-[11px] [&_p]:leading-[1.7] [&_p]:text-[#727c67] [&>.ws-badge]:text-[10px] [&>.ws-badge]:whitespace-nowrap max-[760px]:[&>.ws-badge]:max-w-[115px] max-[760px]:[&>.ws-badge]:[white-space:normal]">
          <div>
            <h2 id="balance-heading" className="font-display">
              Agent balance
            </h2>
            <p>
              {connected
                ? "USDC available in your connected wallet."
                : "Use free capabilities with your GitHub account."}
            </p>
          </div>
          <span className="ws-badge inline-block rounded-[5px] bg-[#f6f8f0] px-[7px] py-1 text-[10px] text-[#686f60] [border:1px_solid_#e2e7d9]">
            {connected ? networkLabel(balance?.chain_id) : "Free access"}
          </span>
        </div>
        <div className="ws-balance-main mb-6 flex flex-wrap items-center justify-between gap-4 [&_.ws-balance-currency]:ml-0 [&>div]:flex [&>div]:flex-wrap [&>div]:items-baseline [&>div]:gap-[9px]">
          <div>
            <strong
              className={
                connected
                  ? "ws-balance-amount font-mono text-[clamp(28px,_3vw,_36px)] font-medium tracking-[-0.07em] text-[color:#303a27]"
                  : "ws-balance-empty font-display text-[22px] font-semibold tracking-[-0.04em] text-[color:#303a27]"
              }
            >
              {typeof amount === "string" ? atomicMoney(amount) : "Free access"}
            </strong>
            {typeof amount === "string" && (
              <span className="ws-balance-currency ml-[9px] text-[11px] text-[#727c67]">
                USDC
              </span>
            )}
          </div>
          <button
            className="ws-button ws-primary inline-flex min-h-[38px] cursor-pointer items-center justify-center gap-[9px] rounded-[999px] bg-white px-[15px] py-[9px] text-center text-[11px] [font-weight:650] shadow-[inset_0_1px_0_#ffffff80,_0_1px_2px_#252a1b04] [border:1px_solid_#dedfd7] disabled:cursor-not-allowed disabled:opacity-[0.45] [&.ws-primary]:[border-color:#20231e] [&.ws-primary]:text-[#fff] [&.ws-primary]:shadow-[inset_0_1px_1px_#ffffff20] [&.ws-primary]:[background:linear-gradient(#353831,_#20231e)] [&.ws-small]:min-h-[33px] [&.ws-small]:px-3 [&.ws-small]:py-[7px] [&.ws-small]:text-[10px] [&:hover]:[border-color:#bbc1ae] [&:hover]:bg-[#40473a] [&:hover]:bg-[#f4f5ee]"
            type="button"
            onClick={() => (connected ? setAction("topup") : onConnect())}
          >
            <Icon name="plus" size={15} />
            {balance?.wallet ? "Top up" : "Add wallet (optional)"}
          </button>
        </div>
        <div className="ws-balance-health is-unknown text-[color:var(--balance-color)] [--balance-color:#6c8554] [&_small]:text-[10px] [&_small]:text-[#8a9180] [&.is-low]:[--balance-color:#bd6353] [&.is-medium]:[--balance-color:#b58b30] [&.is-unknown]:[--balance-color:#a4a99d] [&>div:first-child]:mb-[9px] [&>div:first-child]:flex [&>div:first-child]:items-center [&>div:first-child]:justify-between [&>div:first-child]:gap-2.5 [&>div:first-child]:text-[11px]">
          <div>
            <span>{status}</span>
            <small>
              {connected
                ? networkLabel(balance?.chain_id)
                : "Connect a wallet whenever you want to use paid capabilities."}
            </small>
          </div>
        </div>
        {connected && (
          <div className="ws-balance-actions mt-[18px] flex flex-wrap items-center gap-2.5 pt-4 [border-top:1px_solid_var(--ws-line)] [&>button]:inline-flex [&>button]:min-h-[38px] [&>button]:items-center [&>button]:gap-1.5 [&>button]:rounded-[9px] [&>button]:bg-white [&>button]:px-3.5 [&>button]:py-[9px] [&>button]:text-[12px] [&>button]:text-[#59674d] [&>button]:[border:1px_solid_#dedfd7] [&>button:hover]:text-[#262a22] [&>span]:ml-auto [&>span]:inline-flex [&>span]:items-center [&>span]:gap-1.5 [&>span]:text-[10px] [&>span]:text-[#8a9180] max-[760px]:[&>span]:ml-0 max-[760px]:[&>span]:w-full">
            <button
              type="button"
              onClick={() => setAction("send")}
              className="cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45]"
            >
              <Icon name="arrow" size={15} /> Send
            </button>
            <button
              type="button"
              onClick={() => setAction("receive")}
              className="cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45]"
            >
              <Icon name="down" size={15} /> Receive
            </button>
            <button
              type="button"
              disabled={loading}
              onClick={onRefresh}
              className="cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.45]"
            >
              <Icon name="activity" size={15} /> Refresh
            </button>
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
        <p className="ws-fine mt-3 text-[10px] leading-[1.9] [overflow-wrap:anywhere] text-[#686f60]">
          Session authorization limits and creator earnings are separate from
          the funds held in this wallet.
        </p>
      </section>
      <PaymentConnection balance={balance} token={token} />
      {action && balance?.wallet && (
        <AccountPaymentDialog
          action={action}
          balance={balance}
          onRefresh={onRefresh}
          onClose={() => setAction(null)}
        />
      )}
    </div>
  );
};
