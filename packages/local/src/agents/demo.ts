import { x402Client } from "@x402/core/client";
import type { PaymentRequired } from "@x402/core/types";
import { toClientEvmSigner } from "@x402/evm";
import { ExactEvmScheme } from "@x402/evm/exact/client";
import { erc20Abi, isAddress } from "viem";

import {
  assertLocalChain,
  chain,
  localAccount,
  publicClient,
} from "../config/chain";
import { HiveAgent } from "./client";

await assertLocalChain();
const state: { asset: string } = await Bun.file(
  new URL("../../../../.hive/local/chain.json", import.meta.url)
).json();
if (!isAddress(state.asset)) {
  throw new Error("Run chain/setup.ts first");
}
const baseUrl = process.env.HIVE_DEMO_URL ?? "http://127.0.0.1:3001";
const repository =
  process.env.HIVE_DEMO_REPOSITORY ?? "https://github.com/parazeeknova/hive";
const revision = process.env.HIVE_DEMO_REVISION;
if (!revision || !/^[\da-f]{40}$/iu.test(revision)) {
  throw new Error(
    "Set HIVE_DEMO_REVISION to an immutable 40-character commit SHA"
  );
}
const creatorAccount = localAccount(1);
const buyerAccount = localAccount(2);
const creator = await HiveAgent.connect(baseUrl, creatorAccount, [
  "capabilities:publish",
  "sales:read",
]);
const buyer = await HiveAgent.connect(baseUrl, buyerAccount, [
  "capabilities:read",
  "repository:analyze",
  "receipts:read",
]);
const capability = await creator.json<{ id: string }>("/v1/capabilities", {
  description: "Immutable repository stack analysis",
  name: `Local repository stack ${crypto.randomUUID()}`,
  price: "10000",
  runtime: "repository_stack",
});
const discovered = await buyer.json<{ id: string }[]>("/v1/capabilities");
if (!discovered.some((item) => item.id === capability.id)) {
  throw new Error("Buyer did not discover creator capability");
}
const quote = await buyer.json<{ id: string; payment: PaymentRequired }>(
  "/v1/quotes",
  {
    capability_id: capability.id,
    input: { repository_url: repository, revision },
  },
  { "idempotency-key": crypto.randomUUID() }
);
const paymentClient = new x402Client()
  .setSpendControls({
    allowedAssets: [
      {
        asset: state.asset,
        maxAmountPerPayment: "10000",
        network: `eip155:${chain.id}`,
      },
    ],
  })
  .register(
    `eip155:${chain.id}`,
    new ExactEvmScheme(toClientEvmSigner(buyerAccount, publicClient), {
      rpcUrl: publicClient.transport.url,
    })
  );
const balance = () =>
  publicClient.readContract({
    abi: erc20Abi,
    address: state.asset as `0x${string}`,
    args: [creatorAccount.address],
    functionName: "balanceOf",
  });
const before = await balance();
const payload = await paymentClient.createPaymentPayload(quote.payment);
const signature = Buffer.from(JSON.stringify(payload)).toString("base64");
const receipt = await buyer.json<{
  state: string;
  settlement?: { transaction: string };
}>(`/v1/invocations/${quote.id}`, {}, { "payment-signature": signature });
if (receipt.state !== "settled") {
  throw new Error(
    `Expected settled receipt, received ${JSON.stringify(receipt)}`
  );
}
const after = await balance();
if (after - before !== 10_000n) {
  throw new Error("Creator balance did not increase by the quoted amount");
}
// Replaying a settled invocation returns its receipt without another transfer.
const replay = await buyer.json<{ state: string }>(
  `/v1/invocations/${quote.id}`,
  {},
  { "payment-signature": signature }
);
if (replay.state !== "settled" || (await balance()) !== after) {
  throw new Error("Settled invocation replay charged the buyer twice");
}
const sales = await creator.json<{ invocation_id: string }[]>("/v1/sales");
if (!sales.some((sale) => sale.invocation_id === quote.id)) {
  throw new Error("Creator sales ledger missing invocation");
}
console.log(
  JSON.stringify({
    capability: capability.id,
    creatorEarned: (after - before).toString(),
    invocation: quote.id,
    state: receipt.state,
    transaction: receipt.settlement?.transaction,
  })
);
