"use client";
import Image from "next/image";
import Link from "next/link";
import { useState } from "react";

import { clients } from "./agent-clients";

export const AgentConnect = () => {
  const [selected, setSelected] = useState(0);
  const [status, setStatus] = useState("");
  const client = clients[selected] ?? clients[0];
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(client.command);
      setStatus("Copied. Paste it in your terminal.");
    } catch {
      setStatus("Select the command and copy it to your clipboard.");
    }
  };
  return (
    <div className="agent-connect">
      <div
        className="agent-connect__tabs"
        role="tablist"
        aria-label="Choose your agent"
      >
        {clients.map((item, index) => (
          <button
            key={item.id}
            id={`agent-tab-${item.id}`}
            role="tab"
            type="button"
            aria-selected={selected === index}
            aria-controls="agent-command"
            tabIndex={selected === index ? 0 : -1}
            onKeyDown={(event) => {
              let next = index;
              if (event.key === "ArrowRight") {
                next = (index + 1) % clients.length;
              } else if (event.key === "ArrowLeft") {
                next = (index + clients.length - 1) % clients.length;
              } else if (event.key === "Home") {
                next = 0;
              } else if (event.key === "End") {
                next = clients.length - 1;
              } else {
                return;
              }
              event.preventDefault();
              setSelected(next);
              setStatus("");
              document
                .querySelector<HTMLButtonElement>(
                  `#agent-tab-${clients[next]?.id}`
                )
                ?.focus();
            }}
            onClick={() => {
              setSelected(index);
              setStatus("");
            }}
          >
            <Image
              src={`/clients/${item.id}.svg`}
              alt=""
              width={20}
              height={20}
            />
            {item.name}
          </button>
        ))}
      </div>
      <div
        id="agent-command"
        role="tabpanel"
        aria-labelledby={`agent-tab-${client.id}`}
        tabIndex={0}
      >
        <div className="agent-connect__terminal">
          <span aria-hidden="true">$</span>
          <code>{client.command}</code>
        </div>
        <div className="agent-connect__actions">
          <button className="hive-btn-2" type="button" onClick={copy}>
            Copy command{" "}
            <span className="hive-inline-arrow" aria-hidden="true" />
          </button>
          <Link className="hive-link" href="/docs">
            Setup guide
          </Link>
        </div>
      </div>
      <p className="hive-text-4 agent-connect__note">
        Create an API key in Account settings, then set HIVE_API_KEY in your
        environment.
      </p>
      <output className="hive-text-4 agent-connect__status">{status}</output>
    </div>
  );
};

export const EarningsPreview = () => {
  const [paid, setPaid] = useState(true);
  return (
    <div className="earnings-preview">
      <fieldset>
        <legend className="hive-text-4">Your capability. Your terms.</legend>
        <div className="earnings-preview__modes">
          {[
            { label: "Free to use", value: false },
            { label: "Paid per call", value: true },
          ].map((item) => (
            <label key={item.label} data-selected={paid === item.value}>
              <input
                type="radio"
                name="pricing-preview"
                checked={paid === item.value}
                onChange={() => setPaid(item.value)}
              />
              {item.label}
            </label>
          ))}
        </div>
      </fieldset>
      <div className="earnings-preview__result" aria-live="polite">
        <span>
          100 calls × {paid ? "$0.05" : "$0.00"}
          <small>Example revenue before fees</small>
        </span>
        <strong>{paid ? "$5.00" : "$0.00"}</strong>
      </div>
      <p className="hive-text-4">
        Illustrative pricing. You choose the price when you publish.
      </p>
    </div>
  );
};
