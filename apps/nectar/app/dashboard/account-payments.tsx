"use client";

import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";

import { Icon } from "../../components/icon";
import {
  addDemoFunds,
  balanceHealth,
  parseTopUp,
} from "../../lib/account-store";
import { atomicMoney, errorMessage } from "../../lib/hiveway";

type PaymentAction = "topup" | "send" | "receive";

export const SidebarBalance = ({
  balance,
  preview,
  onOpen,
}: {
  balance: string;
  preview: boolean;
  onOpen: () => void;
}) => {
  const health = balanceHealth(balance);
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
        {preview ? `$${atomicMoney(balance)}` : "Unavailable"}
        <small>{preview ? "USD" : ""}</small>
      </strong>
      <span
        className={`ws-balance-health ${preview ? `is-${health.tone}` : "is-unknown"}`}
        aria-hidden="true"
      >
        <span className="ws-balance-track">
          <span style={{ width: `${preview ? health.percent : 0}%` }} />
        </span>
      </span>
      <span className="ws-sidebar-balance-link">
        Manage balance <Icon name="chevron" size={12} />
      </span>
    </button>
  );
};
const titles: Record<PaymentAction, string> = {
  receive: "Receive funds",
  send: "Send funds",
  topup: "Top up your balance",
};

// Replace this boundary with server-created Dodo checkout and verified webhook crediting.
const AccountPaymentDialog = ({
  action,
  preview,
  onClose,
}: {
  action: PaymentAction;
  preview: boolean;
  onClose: () => void;
}) => {
  const ref = useRef<HTMLDialogElement>(null);
  const [amount, setAmount] = useState("25");
  const [recipient, setRecipient] = useState("");
  const [message, setMessage] = useState("");
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  const topUp = (event: FormEvent) => {
    event.preventDefault();
    if (!preview) {
      setMessage("Checkout is not connected yet.");
      return;
    }
    try {
      addDemoFunds("preview", parseTopUp(amount));
      onClose();
    } catch (error) {
      setMessage(errorMessage(error));
    }
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
          <strong>Dodo Payments</strong>
          <small>Checkout &amp; incoming payments</small>
        </div>
        <span className="ws-badge">Not connected</span>
      </div>
      {action === "topup" && (
        <form onSubmit={topUp}>
          <p className="ws-muted">
            Add funds for your agent to use on paid capabilities.
          </p>
          <div className="ws-topup-presets" aria-label="Top-up amounts">
            {["10", "25", "50", "100"].map((value) => (
              <button
                type="button"
                key={value}
                aria-pressed={amount === value}
                onClick={() => setAmount(value)}
              >
                ${value}
              </button>
            ))}
          </div>
          <label className="ws-field">
            Amount · USD
            <input
              inputMode="decimal"
              value={amount}
              onChange={(event) => setAmount(event.target.value)}
              required
              autoComplete="off"
            />
          </label>
          <p className="ws-fine">
            Dodo checkout is not connected yet. No payment will be taken.
          </p>
          <button type="button" className="ws-button ws-primary" disabled>
            Continue to Dodo checkout <Icon name="external" size={14} />
          </button>
          {preview && (
            <button type="submit" className="ws-button">
              Add demo funds
            </button>
          )}
          {message && (
            <p className="ws-error" role="alert">
              {message}
            </p>
          )}
        </form>
      )}
      {action === "send" && (
        <div className="ws-payment-fields">
          <label className="ws-field">
            Recipient
            <input
              value={recipient}
              onChange={(event) => setRecipient(event.target.value)}
              placeholder="Account or wallet address"
              autoComplete="off"
            />
          </label>
          <label className="ws-field">
            Amount · USD
            <input
              inputMode="decimal"
              value={amount}
              onChange={(event) => setAmount(event.target.value)}
            />
          </label>
          <p className="ws-fine">
            Transfers need a connected payment provider and a supported
            recipient. Sending funds is not available yet.
          </p>
          <button type="button" className="ws-button ws-primary" disabled>
            Review transfer <Icon name="arrow" size={14} />
          </button>
        </div>
      )}
      {action === "receive" && (
        <div className="ws-payment-fields">
          <div className="ws-receive-placeholder">
            <Icon name="down" size={25} />
            <strong>Your incoming payments, in one place.</strong>
            <p>
              Payment links and incoming-payment history will appear here when
              payments are connected.
            </p>
          </div>
          <button type="button" className="ws-button ws-primary" disabled>
            Create payment link <Icon name="external" size={14} />
          </button>
        </div>
      )}
    </dialog>
  );
};

export const AccountBalance = ({
  balance,
  preview,
}: {
  balance: string;
  preview: boolean;
}) => {
  const [action, setAction] = useState<PaymentAction | null>(null);
  const health = balanceHealth(balance);
  return (
    <section className="ws-account-balance" aria-labelledby="balance-heading">
      <div className="ws-account-section-title">
        <div>
          <h2 id="balance-heading">Agent balance</h2>
          <p>Funds available for your agent&apos;s calls.</p>
        </div>
        <span className="ws-badge">
          {preview ? "Preview balance" : "Payments not connected"}
        </span>
      </div>
      <div className="ws-balance-main">
        <div>
          <strong className="ws-balance-amount">
            {preview ? `$${atomicMoney(balance)}` : "—"}
          </strong>
          <span className="ws-balance-currency">USD</span>
        </div>
        <button
          type="button"
          className="ws-button ws-primary"
          onClick={() => setAction("topup")}
        >
          <Icon name="plus" size={15} /> Top up
        </button>
      </div>
      <div
        className={`ws-balance-health ${preview ? `is-${health.tone}` : "is-unknown"}`}
      >
        <div>
          <span>{preview ? health.label : "Balance unavailable"}</span>
          <small>
            {preview
              ? "Balance health"
              : "Connect payments to load your balance"}
          </small>
        </div>
        <div className="ws-balance-track">
          <meter
            className="sr-only"
            min={0}
            max={100}
            low={20}
            high={50}
            optimum={100}
            value={preview ? health.percent : 0}
            aria-label="Balance health relative to $50 starter credit"
            aria-valuetext={
              preview
                ? `$${atomicMoney(balance)} available. ${health.label}`
                : "Balance unavailable"
            }
          />
          <span
            aria-hidden="true"
            style={{ width: `${preview ? health.percent : 0}%` }}
          />
        </div>
      </div>
      <div className="ws-balance-actions">
        <button type="button" onClick={() => setAction("send")}>
          <Icon name="arrow" size={15} /> Send
        </button>
        <button type="button" onClick={() => setAction("receive")}>
          <Icon name="down" size={15} /> Receive
        </button>
      </div>
      {!preview && (
        <p className="ws-fine">
          Your session&apos;s USDC spending limit is separate from this balance.
        </p>
      )}
      {action && (
        <AccountPaymentDialog
          action={action}
          preview={preview}
          onClose={() => setAction(null)}
        />
      )}
    </section>
  );
};
