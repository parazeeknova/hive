import { mkdir } from "node:fs/promises";

import { createWalletClient, http, isAddress } from "viem";

import {
  assertLocalChain,
  chain,
  localAccount,
  publicClient,
  rpcUrl,
} from "../config/chain";
import { compileToken } from "./compile";

await assertLocalChain();
const directory = new URL("../../../../.hive/local/", import.meta.url);
await mkdir(directory, { recursive: true });
const wallet = createWalletClient({
  account: localAccount(0),
  chain,
  transport: http(rpcUrl),
});
const contract = await compileToken();
const state = Bun.file(new URL("chain.json", directory));
let asset: `0x${string}` | undefined;
if (await state.exists()) {
  const previous: { asset?: string } = await state.json();
  if (
    previous.asset &&
    isAddress(previous.asset) &&
    (await publicClient.getCode({ address: previous.asset }))
  ) {
    ({ asset } = previous as { asset: `0x${string}` });
  }
}
if (!asset) {
  const hash = await wallet.deployContract(contract);
  const receipt = await publicClient.waitForTransactionReceipt({ hash });
  asset = receipt.contractAddress ?? undefined;
  if (!asset) {
    throw new Error("Token deployment did not return an address");
  }
}
for (const index of [0, 1, 2]) {
  const account = localAccount(index);
  // oxlint-disable-next-line no-await-in-loop -- Confirm each funding transaction before the next nonce.
  const balance = await publicClient.readContract({
    abi: contract.abi,
    address: asset,
    args: [account.address],
    functionName: "balanceOf",
  });
  if (balance === 0n) {
    // oxlint-disable-next-line no-await-in-loop -- Funding is sequential to avoid nonce races.
    const hash = await wallet.writeContract({
      abi: contract.abi,
      address: asset,
      args: [account.address, 1_000_000_000n],
      functionName: "mint",
    });
    // oxlint-disable-next-line no-await-in-loop -- Require mined funding before starting clients.
    await publicClient.waitForTransactionReceipt({ hash });
  }
}
await Bun.write(
  state,
  JSON.stringify({ asset, chainId: chain.id, rpcUrl }, null, 2)
);
let config = await Bun.file(
  new URL("../../../../apps/hiveway/config.toml", import.meta.url)
).text();
for (const [key, value] of Object.entries({
  asset,
  chain_id: chain.id,
  facilitator_url: "http://127.0.0.1:4022",
  rpc_url: rpcUrl,
})) {
  config = config.replace(
    new RegExp(`^${key} = .+$`, "mu"),
    `${key} = ${JSON.stringify(value)}`
  );
}
await Bun.write(new URL("hiveway.toml", directory), config);
console.log(
  JSON.stringify({
    asset,
    chainId: chain.id,
    message: "Local test chain ready; tokens have no value",
  })
);
