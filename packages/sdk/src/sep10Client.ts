import { NetworkError, ValidationError } from "./errors";

export interface Sep10ChallengeResponse {
  transaction: string;
  network_passphrase?: string;
}

export interface Sep10TokenResponse {
  token: string;
  account: string;
}

export interface Sep10ClientConfig {
  authEndpoint: string;
  networkPassphrase?: string;
  fetchFn?: typeof fetch;
}

export interface DecodedJwtClaims {
  sub?: string;
  iss?: string;
  exp?: number;
  iat?: number;
  [key: string]: any;
}

/**
 * Safely decodes base64url encoded string in both Node.js and browser environments.
 */
function base64UrlDecode(str: string): string {
  let base64 = str.replace(/-/g, "+").replace(/_/g, "/");
  while (base64.length % 4) {
    base64 += "=";
  }
  if (typeof Buffer !== "undefined") {
    return Buffer.from(base64, "base64").toString("utf8");
  }
  return atob(base64);
}

/**
 * Client for SEP-0010 Stellar Web Authentication challenge retrieval and JWT exchange.
 */
export class Sep10Client {
  private readonly authEndpoint: string;
  private readonly networkPassphrase: string;
  private readonly fetchFn: typeof fetch;

  constructor(config: Sep10ClientConfig) {
    if (!config || !config.authEndpoint) {
      throw new ValidationError("authEndpoint is required in Sep10Client configuration");
    }
    this.authEndpoint = config.authEndpoint.replace(/\/+$/, "");
    this.networkPassphrase = config.networkPassphrase || "Test SDF Network ; September 2015";
    this.fetchFn = config.fetchFn || (typeof fetch !== "undefined" ? fetch.bind(globalThis) : (undefined as any));
  }

  /**
   * Request a challenge transaction for the designated Stellar public key.
   */
  public async getChallenge(account: string): Promise<Sep10ChallengeResponse> {
    if (!account) {
      throw new ValidationError("Stellar account public key is required");
    }

    const url = `${this.authEndpoint}/challenge?account=${encodeURIComponent(account)}`;
    try {
      const res = await this.fetchFn(url, {
        method: "GET",
        headers: { Accept: "application/json" },
      });

      if (!res.ok) {
        const errorText = await res.text().catch(() => "");
        throw new NetworkError(`Failed to fetch challenge: HTTP ${res.status} - ${errorText}`);
      }

      const data = await res.json();
      if (!data.transaction) {
        throw new NetworkError("Server did not return a valid 'transaction' challenge field");
      }
      return data;
    } catch (err: any) {
      if (err instanceof NetworkError || err instanceof ValidationError) {
        throw err;
      }
      throw new NetworkError(`Challenge request failed: ${err.message}`);
    }
  }

  /**
   * Submit signed challenge transaction envelope and exchange for a scoped JWT token.
   */
  public async exchangeToken(signedTransactionXdr: string): Promise<Sep10TokenResponse> {
    if (!signedTransactionXdr) {
      throw new ValidationError("signedTransactionXdr is required");
    }

    const url = `${this.authEndpoint}/token`;
    try {
      const res = await this.fetchFn(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify({ transaction: signedTransactionXdr }),
      });

      if (!res.ok) {
        const errorText = await res.text().catch(() => "");
        throw new NetworkError(`Token exchange failed: HTTP ${res.status} - ${errorText}`);
      }

      const data = await res.json();
      if (!data.token) {
        throw new NetworkError("Server did not return a valid session JWT 'token'");
      }
      return data;
    } catch (err: any) {
      if (err instanceof NetworkError || err instanceof ValidationError) {
        throw err;
      }
      throw new NetworkError(`Token exchange request failed: ${err.message}`);
    }
  }

  /**
   * Authenticate end-to-end: fetch challenge, invoke client sign callback, exchange token.
   */
  public async authenticate(
    account: string,
    signCallback: (challengeXdr: string) => Promise<string>
  ): Promise<Sep10TokenResponse> {
    const challenge = await this.getChallenge(account);
    const signedXdr = await signCallback(challenge.transaction);
    return this.exchangeToken(signedXdr);
  }

  /**
   * Parse claims from an encoded JWT session token without verification.
   */
  public static parseJwtClaims(token: string): DecodedJwtClaims {
    if (!token || typeof token !== "string") {
      throw new ValidationError("Token must be a valid non-empty string");
    }
    const parts = token.split(".");
    if (parts.length < 2) {
      throw new ValidationError("Malformed JWT: expected dot-separated segments");
    }
    try {
      const jsonStr = base64UrlDecode(parts[1]);
      return JSON.parse(jsonStr);
    } catch (err: any) {
      throw new ValidationError(`Failed to parse JWT payload: ${err.message}`);
    }
  }
}
