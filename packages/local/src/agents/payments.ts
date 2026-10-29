import { x402Client } from "@x402/core/client";
import { toClientEvmSigner } from "@x402/evm";
import { ExactEvmScheme } from "@x402/evm/exact/client";
import type { LocalAccount } from "viem";

import { chain, publicClient } from "../config/chain";

export const localPaymentClient = (asset: string, buyer: LocalAccount) =>
  new x402Client()
    .setSpendControls({
      allowedAssets: [
        { asset, maxAmountPerPayment: "10000", network: `eip155:${chain.id}` },
      ],
    })
    .register(
      `eip155:${chain.id}`,
      new ExactEvmScheme(toClientEvmSigner(buyer, publicClient), {
        rpcUrl: publicClient.transport.url,
      })
    );
