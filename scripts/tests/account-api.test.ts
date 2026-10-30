/// <reference lib="dom" />
import { afterEach, expect, mock, test } from "bun:test";

import {
  sendUsdc,
  transferData,
  parseTransferAmount,
} from "../../apps/nectar/lib/account";
import type { WalletBalance } from "../../apps/nectar/lib/account";

const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
afterEach(() => {
  mock.restore();
  if (originalWindow) {
    Object.defineProperty(globalThis, "window", originalWindow);
  } else {
    Reflect.deleteProperty(globalThis, "window");
  }
});
const wallet = "0x1111111111111111111111111111111111111111";
const recipient = "0x2222222222222222222222222222222222222222";
const balance: WalletBalance = {
  amount: "1000000",
  asset: "0x3333333333333333333333333333333333333333",
  block: "0xabc",
  chain_id: 84_532,
  decimals: 6,
  dodo_test_checkout: false,
  native_amount: "0",
  network: "eip155:84532",
  wallet,
};
test("transfer encodes the exact recipient and atomic amount", () => {
  expect(transferData(recipient, "123456")).toBe(
    `0xa9059cbb${recipient.slice(2).padStart(64, "0")}${123_456n.toString(16).padStart(64, "0")}`
  );
  for (const [address, amount] of [
    [`0x${"0".repeat(40)}`, "1"],
    [recipient, "0"],
    [recipient, "-1"],
    ["https://evil.invalid", "1"],
  ]) {
    expect(() => transferData(address ?? "", amount ?? "")).toThrow();
  }
});
test("wallet mismatch and wrong chain never submit a transfer", async () => {
  for (const mismatch of ["account", "chain"]) {
    const methods: string[] = [];
    Object.defineProperty(globalThis, "window", {
      configurable: true,
      value: {
        ethereum: {
          request: ({ method }: { method: string }) => {
            methods.push(method);
            if (method === "eth_requestAccounts") {
              return Promise.resolve([
                mismatch === "account" ? recipient : wallet,
              ]);
            }
            if (method === "eth_chainId") {
              return Promise.resolve("0x1");
            }
            return Promise.resolve(null);
          },
        },
      },
    });
    // oxlint-disable-next-line no-await-in-loop -- Each case replaces the shared wallet provider.
    await expect(sendUsdc(balance, recipient, "1")).rejects.toThrow();
    expect(methods).not.toContain("eth_sendTransaction");
  }
});
test("a matching wallet reviews the configured token transfer", async () => {
  let transaction: unknown;
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      ethereum: {
        request: ({
          method,
          params,
        }: {
          method: string;
          params?: unknown[];
        }) => {
          if (method === "eth_requestAccounts") {
            return Promise.resolve([wallet]);
          }
          if (method === "eth_chainId") {
            return Promise.resolve("0x14a34");
          }
          if (method === "eth_sendTransaction") {
            transaction = params?.[0];
            return Promise.resolve(`0x${"a".repeat(64)}`);
          }
          return Promise.resolve(null);
        },
      },
    },
  });
  expect(await sendUsdc(balance, recipient, "1000000")).toBe(
    `0x${"a".repeat(64)}`
  );
  expect(transaction).toEqual({
    data: transferData(recipient, "1000000"),
    from: wallet,
    to: balance.asset,
    value: "0x0",
  });
});

test("transfer amounts allow balances above capability pricing limits", () => {
  expect(parseTransferAmount("1001.123456")).toBe("1001123456");
  for (const value of ["0", "-1", "1.0000001", "1e6", "9".repeat(86)]) {
    expect(() => parseTransferAmount(value)).toThrow();
  }
});
