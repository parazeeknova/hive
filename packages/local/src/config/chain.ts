import { defineChain, http, createPublicClient } from "viem";
import { mnemonicToAccount } from "viem/accounts";

// Public Anvil fixture mnemonic. No real funds or public-network signing.
const mnemonic = "test test test test test test test test test test test junk";
export const rpcUrl = process.env.HIVE_LOCAL_RPC_URL ?? "http://127.0.0.1:8545";
export const chain = defineChain({
  id: 31_337,
  name: "Hive isolated development chain",
  nativeCurrency: { decimals: 18, name: "Test Ether", symbol: "ETH" },
  rpcUrls: { default: { http: [rpcUrl] } },
});
export const publicClient = createPublicClient({
  chain,
  transport: http(rpcUrl),
});
export const localAccount = (index: number) =>
  mnemonicToAccount(mnemonic, { addressIndex: index });
export const assertLocalChain = async () => {
  const url = new URL(rpcUrl);
  if (
    url.protocol !== "http:" ||
    !["127.0.0.1", "localhost", "anvil"].includes(url.hostname)
  ) {
    throw new Error(
      "Development wallets require an isolated local RPC endpoint"
    );
  }
  const version = await publicClient.request({ method: "web3_clientVersion" });
  if (
    (await publicClient.getChainId()) !== chain.id ||
    !version.toLowerCase().includes("anvil")
  ) {
    throw new Error("Development wallets require local Anvil chain 31337");
  }
};
