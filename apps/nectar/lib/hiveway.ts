/** Hiveway's current REST contract. Amounts remain atomic decimal strings. */
export type Runtime = "repository_stack" | "dependency_inventory" | "workflow";
export interface RegistryCapability {
  id: string;
  creator_wallet: string;
  name: string;
  description: string;
  version: string;
  runtime: Runtime;
  price: string;
  active: boolean;
  definition: Record<string, unknown>;
}
export interface Credential {
  id: string;
  wallet: string;
  name: string;
  scopes: string[];
  per_call_limit: string;
  total_limit: string;
  spent: string;
  reserved: string;
  expires_at: number;
}
export interface Invocation {
  id: string;
  capability_id: string;
  state: string;
  price: string;
  payment_issue: string | null;
  execution_ms: number | null;
  result_bytes: number | null;
  created_at: number;
  completed_at: number | null;
  payment_method: "x402" | "dodo" | null;
}
export interface Receipt {
  id: string;
  capability_id: string;
  state: string;
  price: string;
  result: unknown;
  error_code: string | null;
  payment_issue: string | null;
  settlement: { transaction: string; network: string } | null;
}
export interface Activity {
  invocations: Invocation[];
  budget: { limit: string; spent: string; reserved: string };
}
export interface Sale {
  invocation_id: string;
  payer_wallet: string;
  amount: string;
  network: string;
  transaction_hash: string;
  created_at: number;
}
export interface Quote {
  id: string;
  expires_at: number;
  payment: {
    accepts: {
      amount: string;
      network: string;
      asset: string;
      payTo: string;
    }[];
  };
}
export const runtimeLabels: Record<Runtime, string> = {
  dependency_inventory: "Dependencies",
  repository_stack: "Stack report",
  workflow: "Workflow",
};
export const atomicMoney = (value: string) => {
  const amount = BigInt(value);
  const fraction = (amount % 1_000_000n)
    .toString()
    .padStart(6, "0")
    .replace(/0+$/u, "");
  return `${amount / 1_000_000n}.${fraction.padEnd(2, "0")}`;
};
export const parsePrice = (value: string) => {
  if (!/^\d+(?:\.\d{1,6})?$/u.test(value)) {
    throw new Error("Enter a USDC amount with at most six decimal places.");
  }
  const [whole = "0", fraction = ""] = value.split(".");
  const amount = BigInt(whole) * 1_000_000n + BigInt(fraction.padEnd(6, "0"));
  if (amount < 1n || amount > 1_000_000_000n) {
    throw new Error("Price must be between 0.000001 and 1,000 USDC.");
  }
  return amount.toString();
};
export const validateRepository = (repository: string, revision: string) => {
  if (!/^https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/?$/u.test(repository)) {
    throw new Error(
      "Use a public repository URL: https://github.com/owner/repo."
    );
  }
  if (!/^[a-f\d]{40}$/iu.test(revision)) {
    throw new Error(
      "Enter the full 40-character commit SHA. Branches and tags are not supported."
    );
  }
};
export class HivewayError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = "HivewayError";
    this.status = status;
  }
}
export const apiOrigin = (
  process.env.NEXT_PUBLIC_HIVEWAY_ORIGIN ?? "http://127.0.0.1:3001"
).replace(/\/$/u, "");
export const hiveway = async <T>(
  path: string,
  token = "",
  init: RequestInit = {}
): Promise<T> => {
  const headers = new Headers(init.headers);
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }
  if (init.body) {
    headers.set("Content-Type", "application/json");
  }
  const response = await fetch(`${apiOrigin}${path}`, {
    ...init,
    cache: "no-store",
    headers,
  });
  const raw = await response.text();
  let body: unknown = null;
  try {
    body = raw ? JSON.parse(raw) : null;
  } catch {
    body = raw;
  }
  // Terminal receipts and x402 requirements are ordinary response bodies.
  if (
    !response.ok &&
    !(
      response.status === 409 &&
      body &&
      typeof body === "object" &&
      "state" in body &&
      "capability_id" in body
    ) &&
    response.status !== 402
  ) {
    let message = `Request failed (${response.status})`;
    if (typeof body === "string" && body) {
      message = body;
    }
    if (body && typeof body === "object" && "error" in body) {
      message = String(body.error);
    }
    throw new HivewayError(response.status, message);
  }
  return body as T;
};
export const errorMessage = (error: unknown) =>
  error instanceof Error
    ? error.message.replaceAll("_", " ")
    : "Unable to complete the request.";
export const shortId = (value: string) =>
  value.length > 18 ? `${value.slice(0, 8)}…${value.slice(-6)}` : value;
export const terminalStates = new Set([
  "settled",
  "failed",
  "expired",
  "execution_failed",
]);

// Deliberately separate from the registry. No simulated balance or verification.
export const exampleCapabilities: RegistryCapability[] = [
  {
    active: true,
    creator_wallet: "Example creator",
    definition: {},
    description:
      "Understand a public repository’s declared technologies and manifest files at a specific commit.",
    id: "example-stack",
    name: "Repository stack report",
    price: "10000",
    runtime: "repository_stack",
    version: "1",
  },
  {
    active: true,
    creator_wallet: "Example creator",
    definition: {},
    description:
      "Get structured dependencies, ecosystems, and version requirements from repository manifests.",
    id: "example-dependencies",
    name: "Dependency inventory",
    price: "10000",
    runtime: "dependency_inventory",
    version: "1",
  },
  {
    active: true,
    creator_wallet: "Example creator",
    definition: { steps: ["example-stack", "example-dependencies"] },
    description:
      "Bundle stack and dependency reports for the same immutable repository revision.",
    id: "example-workflow",
    name: "Repository overview",
    price: "20000",
    runtime: "workflow",
    version: "1",
  },
];

export const validatePublishText = (name: string, description: string) => {
  if (
    !name.trim() ||
    !description.trim() ||
    new TextEncoder().encode(name).length > 80 ||
    new TextEncoder().encode(description).length > 1000
  ) {
    throw new Error(
      "Use a nonempty name within 80 bytes and description within 1,000 bytes."
    );
  }
};

export const issueWalletSession = async (creator: boolean) => {
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
    throw new Error(
      "Open this page with an EVM wallet extension, or use an existing credential below."
    );
  }
  const [wallet] = (await provider.request({
    method: "eth_requestAccounts",
  })) as string[];
  if (!wallet) {
    throw new Error("No wallet account selected.");
  }
  const scopes = [
    "capabilities:read",
    "repository:analyze",
    "receipts:read",
    ...(creator
      ? ["capabilities:publish", "workflows:publish", "sales:read"]
      : []),
  ];
  const challenge = await hiveway<{ id: string; message: string }>(
    "/v1/auth/challenges",
    "",
    {
      body: JSON.stringify({
        action: {
          policy: {
            expires_at: Math.floor(Date.now() / 1000) + 3600,
            name: "Nectar session",
            per_call_limit: "1000000",
            scopes,
            total_limit: "10000000",
          },
          type: "issue",
        },
        wallet,
      }),
      method: "POST",
    }
  );
  const encoded = `0x${Array.from(new TextEncoder().encode(challenge.message), (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
  const signature = await provider.request({
    method: "personal_sign",
    params: [encoded, wallet],
  });
  const auth = await hiveway<{ token: string | null }>("/v1/auth/verify", "", {
    body: JSON.stringify({ challenge_id: challenge.id, signature }),
    method: "POST",
  });
  if (!auth.token) {
    throw new Error("The wallet did not issue a credential.");
  }
  return auth.token;
};
