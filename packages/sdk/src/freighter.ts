import { ValidationError } from "./errors";

export interface FreighterSignOptions {
  networkPassphrase?: string;
  network?: "PUBLIC" | "TESTNET";
  accountToSign?: string;
}

export interface FreighterApi {
  isConnected: () => Promise<boolean>;
  isAllowed?: () => Promise<boolean>;
  setAllowed?: () => Promise<boolean>;
  getUserInfo: () => Promise<{ publicKey: string }>;
  getNetworkDetails?: () => Promise<{ network: string; networkPassphrase?: string }>;
  signTransaction: (
    xdr: string,
    opts?: { networkPassphrase?: string; network?: string; accountToSign?: string }
  ) => Promise<string>;
}

export enum WalletConnectionStatus {
  Connected = "CONNECTED",
  NotInstalled = "NOT_INSTALLED",
  Locked = "LOCKED",
  Unauthorized = "UNAUTHORIZED",
  NetworkMismatch = "NETWORK_MISMATCH",
}

export class FreighterWalletError extends Error {
  public code: string;
  constructor(message: string, code: string) {
    super(message);
    this.name = "FreighterWalletError";
    this.code = code;
  }
}

/**
 * Signs an XDR transaction envelope using the Freighter browser wallet API.
 *
 * @param xdr - The base64-encoded XDR transaction envelope to sign
 * @param options - Signing options including networkPassphrase and target account
 * @param customApi - Optional Freighter API instance (for testing or direct injection)
 * @returns The signed transaction envelope XDR
 */
export async function signWithFreighter(
  xdr: string,
  options?: FreighterSignOptions,
  customApi?: FreighterApi
): Promise<string> {
  if (!xdr || typeof xdr !== "string") {
    throw new ValidationError("Transaction envelope XDR must be a non-empty string");
  }

  let api: FreighterApi | undefined = customApi;
  if (!api && typeof window !== "undefined") {
    api = (window as any).freighter;
  }

  if (!api) {
    throw new FreighterWalletError(
      "Freighter wallet is not installed or available in this environment",
      WalletConnectionStatus.NotInstalled
    );
  }

  const isConnected = await api.isConnected();
  if (!isConnected) {
    throw new FreighterWalletError(
      "Freighter wallet is locked or not authorized",
      WalletConnectionStatus.Locked
    );
  }

  if (options?.networkPassphrase && api.getNetworkDetails) {
    try {
      const details = await api.getNetworkDetails();
      if (details?.networkPassphrase && details.networkPassphrase !== options.networkPassphrase) {
        throw new FreighterWalletError(
          `Network mismatch: Freighter is on ${details.networkPassphrase}, but transaction requires ${options.networkPassphrase}`,
          WalletConnectionStatus.NetworkMismatch
        );
      }
    } catch (err: any) {
      if (err instanceof FreighterWalletError) throw err;
    }
  }

  try {
    const signedXdr = await api.signTransaction(xdr, {
      networkPassphrase: options?.networkPassphrase,
      network: options?.network,
      accountToSign: options?.accountToSign,
    });

    if (!signedXdr || typeof signedXdr !== "string") {
      throw new FreighterWalletError("User rejected the signing request", "USER_REJECTED");
    }

    return signedXdr;
  } catch (err: any) {
    if (err instanceof FreighterWalletError) throw err;
    const msg = err?.message || String(err);
    if (msg.toLowerCase().includes("user rejected") || msg.toLowerCase().includes("declined")) {
      throw new FreighterWalletError("Transaction signing rejected by user", "USER_REJECTED");
    }
    throw new FreighterWalletError(`Failed to sign with Freighter: ${msg}`, "SIGNING_FAILED");
  }
}
