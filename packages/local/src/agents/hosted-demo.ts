import type { PaymentRequired } from "@x402/core/types";
import { erc20Abi, isAddress } from "viem";

import { assertLocalChain, localAccount, publicClient } from "../config/chain";
import { HiveAgent } from "./client";
import { localPaymentClient } from "./payments";

await assertLocalChain();
const fixtureRoot = new URL("../../../../.hive/local/", import.meta.url);
const state: { asset: string } = await Bun.file(
  new URL("chain.json", fixtureRoot)
).json();
if (!isAddress(state.asset)) {
  throw new Error("Run chain/setup.ts first");
}
const compiler = Bun.spawn(
  [
    "cargo",
    "run",
    "--locked",
    "--example",
    "compile_wasm",
    "--",
    "packages/local/fixtures/wasm/echo.wat",
    new URL("echo.wasm", fixtureRoot).pathname,
  ],
  { stderr: "inherit", stdout: "inherit" }
);
if ((await compiler.exited) !== 0) {
  throw new Error("Wasm fixture compilation failed");
}
const baseUrl = process.env.HIVE_DEMO_URL ?? "http://127.0.0.1:3001";
const creatorAccount = localAccount(1);
const buyerAccount = localAccount(2);
const creator = await HiveAgent.connect(baseUrl, creatorAccount, [
  "capabilities:publish",
  "sales:read",
]);
const buyer = await HiveAgent.connect(baseUrl, buyerAccount, [
  "capabilities:read",
  "capabilities:invoke",
  "receipts:read",
]);
const code = await Bun.file(new URL("echo.wasm", fixtureRoot)).arrayBuffer();
const schema = {
  additionalProperties: false,
  properties: { message: { type: "string" } },
  required: ["message"],
  type: "object",
};
const artifact = await creator.json<{ id: string }>("/v1/artifacts", {
  input_schema: schema,
  output_schema: schema,
  wasm_base64: Buffer.from(code).toString("base64"),
});
const capability = await creator.json<{ id: string }>("/v1/capabilities", {
  artifact_id: artifact.id,
  description: "Pure JSON hosted Wasm; dynamically echoes each request",
  name: "JSON echo",
  price: "10000",
  runtime: "wasm",
});
const discovered = await buyer.json<{ id: string }[]>(
  "/v1/capabilities?q=JSON%20echo"
);
if (!discovered.some((item) => item.id === capability.id)) {
  throw new Error("Hosted tool not discovered");
}
const paymentClient = localPaymentClient(state.asset, buyerAccount);
const balance = () =>
  publicClient.readContract({
    abi: erc20Abi,
    address: state.asset as `0x${string}`,
    args: [creatorAccount.address],
    functionName: "balanceOf",
  });
const before = await balance();
const invocations: string[] = [];
// Payments are sequential so each balance/replay assertion observes only its own transfer.
/* eslint-disable no-await-in-loop */
for (const message of ["first dynamic input", "a different dynamic input"]) {
  const input = { message };
  const quote = await buyer.json<{ id: string; payment: PaymentRequired }>(
    "/v1/quotes",
    { capability_id: capability.id, input },
    { "idempotency-key": crypto.randomUUID() }
  );
  const payload = await paymentClient.createPaymentPayload(quote.payment);
  const headers = {
    "payment-signature": Buffer.from(JSON.stringify(payload)).toString(
      "base64"
    ),
  };
  const receipt = await buyer.json<{ state: string; result: unknown }>(
    `/v1/invocations/${quote.id}`,
    {},
    headers
  );
  if (
    receipt.state !== "settled" ||
    JSON.stringify(receipt.result) !== JSON.stringify(input)
  ) {
    throw new Error(`Unexpected hosted result: ${JSON.stringify(receipt)}`);
  }
  const after = await balance();
  await buyer.json(`/v1/invocations/${quote.id}`, {}, headers);
  if ((await balance()) !== after) {
    throw new Error("Replay charged twice");
  }
  invocations.push(quote.id);
}
/* eslint-enable no-await-in-loop */
const earned = (await balance()) - before;
if (earned !== 20_000n) {
  throw new Error("Incorrect direct creator payout");
}
const sales = await creator.json<{ invocation_id: string }[]>("/v1/sales");
if (
  !invocations.every((id) => sales.some((sale) => sale.invocation_id === id))
) {
  throw new Error("Hosted creator sales missing");
}
console.log(
  JSON.stringify({
    artifact: artifact.id,
    capability: capability.id,
    creatorEarned: earned.toString(),
    dynamicInputs: 2,
    invocations,
    state: "settled",
  })
);
