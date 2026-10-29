import { x402Facilitator } from "@x402/core/facilitator";
import {
  PaymentPayloadV2Schema,
  PaymentRequirementsV2Schema,
} from "@x402/core/schemas";
import type { PaymentPayload, PaymentRequirements } from "@x402/core/types";
import { toFacilitatorEvmSigner } from "@x402/evm";
import { ExactEvmScheme } from "@x402/evm/exact/facilitator";
import { createWalletClient, http, publicActions } from "viem";
import type { TypedData } from "viem";

import {
  assertLocalChain,
  chain,
  localAccount,
  rpcUrl,
  publicClient,
} from "../config/chain";

await assertLocalChain();
const account = localAccount(0);
const client = createWalletClient({
  account,
  chain,
  transport: http(rpcUrl),
}).extend(publicActions);
const signer = toFacilitatorEvmSigner(
  {
    address: account.address,
    getCode: client.getCode,
    readContract: client.readContract,
    sendTransaction: client.sendTransaction,
    verifyTypedData: (args) =>
      publicClient.verifyTypedData({
        ...args,
        blockTag: "latest",
        types: args.types as TypedData,
      }),
    waitForTransactionReceipt: client.waitForTransactionReceipt,
    writeContract: client.writeContract,
  },
  { confirmationTimeoutMs: 20_000 }
);
const facilitator = new x402Facilitator().register(
  `eip155:${chain.id}`,
  new ExactEvmScheme(signer, { simulateInSettle: true })
);
const server = Bun.serve({
  async fetch(request) {
    const path = new URL(request.url).pathname;
    if (request.method === "GET" && path === "/supported") {
      return Response.json(facilitator.getSupported());
    }
    if (request.method !== "POST" || !["/verify", "/settle"].includes(path)) {
      return new Response(null, { status: 404 });
    }
    try {
      const input: unknown = await request.json();
      if (
        typeof input !== "object" ||
        input === null ||
        !("paymentPayload" in input) ||
        !("paymentRequirements" in input)
      ) {
        throw new Error("Invalid request");
      }
      const body = {
        paymentPayload: PaymentPayloadV2Schema.parse(
          input.paymentPayload
        ) as PaymentPayload,
        paymentRequirements: PaymentRequirementsV2Schema.parse(
          input.paymentRequirements
        ) as PaymentRequirements,
      };
      if (
        body.paymentRequirements.network !== `eip155:${chain.id}` ||
        body.paymentPayload.x402Version !== 2
      ) {
        return Response.json(
          { error: "unsupported_local_network" },
          { status: 400 }
        );
      }
      return Response.json(
        path === "/verify"
          ? await facilitator.verify(
              body.paymentPayload,
              body.paymentRequirements
            )
          : await facilitator.settle(
              body.paymentPayload,
              body.paymentRequirements
            )
      );
    } catch {
      return Response.json(
        { error: "invalid_local_payment_request" },
        { status: 400 }
      );
    }
  },
  hostname: process.env.HIVE_FACILITATOR_HOST ?? "127.0.0.1",
  maxRequestBodySize: 16_384,
  port: Number(process.env.HIVE_FACILITATOR_PORT ?? 4022),
});
console.log(
  `Hive development facilitator listening on ${server.hostname}:${server.port}; isolated chain only`
);
