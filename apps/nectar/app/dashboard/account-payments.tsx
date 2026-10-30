"use client";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";

import { Icon } from "../../components/icon";
import type { WalletBalance } from "../../lib/account";
import { sendUsdc, networkLabel } from "../../lib/account";
import {
  atomicMoney,
  errorMessage,
  hiveway,
  parsePrice,
  shortId,
} from "../../lib/hiveway";

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
      className="ws-sidebar-balance"
      type="button"
      onClick={onOpen}
      aria-label="Open agent balance in Account settings"
    >
      <span className="ws-sidebar-balance-label">
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
      <span className="ws-sidebar-balance-link">
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
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setMessage("");
    try {
      setHash(await sendUsdc(balance, recipient.trim(), parsePrice(amount)));
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
      className="ws-dialog ws-funding-dialog"
      onClose={onClose}
      aria-labelledby="funding-title"
    >
      <div className="ws-dialog-heading">
        <div>
          <span className="ws-overline">AGENT BALANCE</span>
          <h2 id="funding-title">{titles[action]}</h2>
        </div>
        <button
          type="button"
          className="ws-icon-button"
          onClick={onClose}
          aria-label="Close payment dialog"
        >
          <Icon name="close" size={18} />
        </button>
      </div>
      <div className="ws-payment-provider">
        <span className="ws-payment-provider-icon">
          <Icon name="wallet" size={21} />
        </span>
        <div>
          <strong>Connected wallet</strong>
          <small>{networkLabel(balance.chain_id)}</small>
        </div>
      </div>
      {action === "send" ? (
        <form
          className="ws-payment-fields"
          onSubmit={(event) => {
            void submit(event);
          }}
        >
          <label className="ws-field">
            Recipient wallet
            <input
              required
              value={recipient}
              onChange={(event) => setRecipient(event.target.value)}
              placeholder="0x…"
              autoComplete="off"
            />
          </label>
          <label className="ws-field">
            Amount · USDC
            <input
              required
              inputMode="decimal"
              value={amount}
              onChange={(event) => setAmount(event.target.value)}
            />
          </label>
          <p className="ws-fine">
            Your wallet will show the recipient, token amount, and network fee
            before you approve.
          </p>
          <button
            type="submit"
            className="ws-button ws-primary"
            disabled={busy || Boolean(hash)}
          >
            {busy ? "Waiting for wallet…" : "Review in wallet"}
            <Icon name="arrow" size={14} />
          </button>
        </form>
      ) : (
        <div className="ws-payment-fields">
          <p className="ws-muted">
            Send USDC on {networkLabel(balance.chain_id)} to this wallet
            address. Funds remain in your wallet and can be used for capability
            payments.
          </p>
          <code className="ws-receive-address">{balance.wallet}</code>
          <button
            className="ws-button"
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
              className="ws-button ws-primary"
              href="https://faucet.circle.com/"
              target="_blank"
              rel="noreferrer"
            >
              Get free Base Sepolia test USDC <Icon name="external" size={14} />
            </a>
          )}
          <p className="ws-fine">
            Dodo checkouts pay for individual invocations. They do not credit a
            wallet balance.
          </p>
        </div>
      )}
      {hash && (
        <>
          <code className="ws-receive-address">{hash}</code>
          <button className="ws-button" type="button" onClick={onRefresh}>
            Refresh balance
          </button>
        </>
      )}
      {message && <output className="ws-fine">{message}</output>}
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
    <aside className="ws-account-secondary">
      <div className="ws-account-section-title">
        <div>
          <h2>Payment connection</h2>
          <p>Funding and incoming payments.</p>
        </div>
      </div>
      <div className="ws-account-provider">
        <span className="ws-payment-provider-icon">
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
      <dl className="ws-account-payment-facts">
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
        <p className="ws-fine">{errorMessage(earnings.error)}</p>
      )}
      <p className="ws-fine">
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
    <div className="ws-account-split">
      <section className="ws-account-balance" aria-labelledby="balance-heading">
        <div className="ws-account-section-title">
          <div>
            <h2 id="balance-heading">Agent balance</h2>
            <p>
              {connected
                ? "USDC available in your connected wallet."
                : "Use free capabilities with your GitHub account."}
            </p>
          </div>
          <span className="ws-badge">
            {connected ? networkLabel(balance?.chain_id) : "Free access"}
          </span>
        </div>
        <div className="ws-balance-main">
          <div>
            <strong
              className={connected ? "ws-balance-amount" : "ws-balance-empty"}
            >
              {typeof amount === "string" ? atomicMoney(amount) : "Free access"}
            </strong>
            {typeof amount === "string" && (
              <span className="ws-balance-currency">USDC</span>
            )}
          </div>
          <button
            className="ws-button ws-primary"
            type="button"
            onClick={() => (connected ? setAction("topup") : onConnect())}
          >
            <Icon name="plus" size={15} />
            {balance?.wallet ? "Top up" : "Add wallet (optional)"}
          </button>
        </div>
        <div className="ws-balance-health is-unknown">
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
          <div className="ws-balance-actions">
            <button type="button" onClick={() => setAction("send")}>
              <Icon name="arrow" size={15} /> Send
            </button>
            <button type="button" onClick={() => setAction("receive")}>
              <Icon name="down" size={15} /> Receive
            </button>
            <button type="button" disabled={loading} onClick={onRefresh}>
              <Icon name="activity" size={15} /> Refresh
            </button>
          </div>
        )}
        {error && (
          <p className="ws-error" role="alert">
            {error}
          </p>
        )}
        <p className="ws-fine">
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
