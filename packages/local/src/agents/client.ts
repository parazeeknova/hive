import type { LocalAccount } from "viem";

interface Challenge {
  id: string;
  message: string;
}
interface IssuedCredential {
  credential_id: string;
  token: string;
}
export class HiveAgent {
  readonly baseUrl: string;
  readonly token: string;
  constructor(baseUrl: string, token: string) {
    this.baseUrl = baseUrl;
    this.token = token;
  }

  static async connect(
    baseUrl: string,
    account: LocalAccount,
    scopes: string[]
  ) {
    const request = async <T>(path: string, body: unknown): Promise<T> => {
      const response = await fetch(new URL(path, baseUrl), {
        body: JSON.stringify(body),
        headers: { "content-type": "application/json" },
        method: "POST",
        signal: AbortSignal.timeout(30_000),
      });
      if (!response.ok) {
        throw new Error(
          `Hiveway ${path}: ${response.status} ${await response.text()}`
        );
      }
      return (await response.json()) as T;
    };
    const challenge = await request<Challenge>("/v1/auth/challenges", {
      action: {
        policy: {
          expires_at: Math.floor(Date.now() / 1000) + 3600,
          name: "local demo agent",
          per_call_limit: "100000",
          scopes,
          total_limit: "1000000",
        },
        type: "issue",
      },
      wallet: account.address,
    });
    const signature = await account.signMessage({ message: challenge.message });
    const credential = await request<IssuedCredential>("/v1/auth/verify", {
      challenge_id: challenge.id,
      signature,
    });
    if (!credential.token) {
      throw new Error("Credential issuance returned no token");
    }
    return new HiveAgent(baseUrl, credential.token);
  }

  request(path: string, body?: unknown, headers?: Record<string, string>) {
    return fetch(new URL(path, this.baseUrl), {
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      headers: {
        authorization: `Bearer ${this.token}`,
        "content-type": "application/json",
        ...headers,
      },
      method: body === undefined ? "GET" : "POST",
      signal: AbortSignal.timeout(240_000),
    });
  }
  async json<T>(
    path: string,
    body?: unknown,
    headers?: Record<string, string>
  ): Promise<T> {
    const response = await this.request(path, body, headers);
    if (!response.ok) {
      throw new Error(
        `Hiveway ${path}: ${response.status} ${await response.text()}`
      );
    }
    return (await response.json()) as T;
  }
}
