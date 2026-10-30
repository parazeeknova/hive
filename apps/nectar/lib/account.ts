export interface Profile {
  name: string;
  username: string;
  avatar: string;
}
export interface WalletBalance {
  wallet: string | null;
  amount: string | null;
  native_amount: string | null;
  block: string | null;
  chain_id: number;
  network: string;
  asset: string;
  decimals: number;
  dodo_test_checkout: boolean;
}
export const transferData = (recipient: string, amount: string) => {
  if (!/^0x[\da-f]{40}$/iu.test(recipient) || /^0x0{40}$/iu.test(recipient)) {
    throw new Error("Enter a valid recipient wallet address.");
  }
  if (
    !/^\d+$/u.test(amount) ||
    BigInt(amount) <= 0n ||
    BigInt(amount) >= 2n ** 256n
  ) {
    throw new Error("Enter a positive USDC amount.");
  }
  return `0xa9059cbb${recipient.slice(2).toLowerCase().padStart(64, "0")}${BigInt(amount).toString(16).padStart(64, "0")}`;
};
export const walletProvider = () => {
  const provider = (
    window as unknown as {
      ethereum?: {
        request: (args: {
          method: string;
          params?: unknown[];
        }) => Promise<unknown>;
      };
    }
  ).ethereum;
  if (!provider) {
    throw new Error("Open this page with an EVM wallet extension.");
  }
  return provider;
};
export const sendUsdc = async (
  balance: WalletBalance,
  recipient: string,
  amount: string
) => {
  if (
    !balance.wallet ||
    balance.amount === null ||
    BigInt(amount) > BigInt(balance.amount)
  ) {
    throw new Error("Insufficient wallet USDC balance.");
  }
  const data = transferData(recipient, amount);
  const provider = walletProvider();
  const accounts = (await provider.request({
    method: "eth_requestAccounts",
  })) as string[];
  if (accounts[0]?.toLowerCase() !== balance.wallet.toLowerCase()) {
    throw new Error(
      "Select the wallet used by your connected Hive credential."
    );
  }
  await provider.request({
    method: "wallet_switchEthereumChain",
    params: [{ chainId: `0x${balance.chain_id.toString(16)}` }],
  });
  const chain = await provider.request({ method: "eth_chainId" });
  if (chain !== `0x${balance.chain_id.toString(16)}`) {
    throw new Error("Wallet is on the wrong network.");
  }
  const hash = await provider.request({
    method: "eth_sendTransaction",
    params: [{ data, from: balance.wallet, to: balance.asset, value: "0x0" }],
  });
  if (typeof hash !== "string" || !/^0x[\da-f]{64}$/iu.test(hash)) {
    throw new Error("Wallet did not return a transaction hash.");
  }
  return hash;
};

export const networkLabel = (chainId: number | undefined) => {
  if (chainId === 84_532) {
    return "Base Sepolia · testnet";
  }
  if (chainId === 8453) {
    return "Base";
  }
  if (chainId === 31_337) {
    return "Local test network";
  }
  return "Wallet network";
};
