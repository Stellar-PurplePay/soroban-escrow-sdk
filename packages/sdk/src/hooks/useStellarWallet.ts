import { FreighterApi, FreighterSignOptions, signWithFreighter, FreighterWalletError } from "../freighter";

export interface StellarWalletState {
  publicKey: string | null;
  network: string | null;
  networkPassphrase: string | null;
  isConnected: boolean;
  isConnecting: boolean;
  error: Error | null;
}

/**
 * State manager and reactive coordinator for Stellar wallet (e.g. Freighter) interactions.
 */
export class StellarWalletManager {
  private api?: FreighterApi;
  private state: StellarWalletState;
  private listeners: Array<(state: StellarWalletState) => void> = [];

  constructor(customApi?: FreighterApi) {
    this.api = customApi;
    this.state = {
      publicKey: null,
      network: null,
      networkPassphrase: null,
      isConnected: false,
      isConnecting: false,
      error: null,
    };
  }

  public getState(): StellarWalletState {
    return { ...this.state };
  }

  public subscribe(listener: (state: StellarWalletState) => void): () => void {
    this.listeners.push(listener);
    listener(this.getState());
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notify() {
    const current = this.getState();
    for (const listener of this.listeners) {
      listener(current);
    }
  }

  public async connect(overrideApi?: FreighterApi): Promise<string> {
    if (overrideApi) {
      this.api = overrideApi;
    }

    let activeApi = this.api;
    if (!activeApi && typeof window !== "undefined") {
      activeApi = (window as any).freighter;
    }

    if (!activeApi) {
      const err = new FreighterWalletError(
        "Freighter extension is not installed or available in the global environment",
        "NOT_INSTALLED"
      );
      this.state.error = err;
      this.notify();
      throw err;
    }

    this.state.isConnecting = true;
    this.state.error = null;
    this.notify();

    try {
      const isConnected = await activeApi.isConnected();
      if (!isConnected) {
        throw new FreighterWalletError("Freighter wallet is locked or unavailable", "LOCKED");
      }

      if (activeApi.isAllowed && !(await activeApi.isAllowed())) {
        if (activeApi.setAllowed) {
          const allowed = await activeApi.setAllowed();
          if (!allowed) {
            throw new FreighterWalletError("User rejected Freighter connection authorization", "UNAUTHORIZED");
          }
        }
      }

      const userInfo = await activeApi.getUserInfo();
      if (!userInfo || !userInfo.publicKey) {
        throw new FreighterWalletError("Failed to retrieve public key from Freighter wallet", "NO_KEY");
      }

      let networkDetails: { network: string; networkPassphrase?: string } = { network: "TESTNET" };
      if (activeApi.getNetworkDetails) {
        try {
          networkDetails = await activeApi.getNetworkDetails();
        } catch {
          // Fallback to default
        }
      }

      this.state = {
        publicKey: userInfo.publicKey,
        network: networkDetails.network,
        networkPassphrase: networkDetails.networkPassphrase || null,
        isConnected: true,
        isConnecting: false,
        error: null,
      };
      this.notify();
      return userInfo.publicKey;
    } catch (err: any) {
      const errorObj = err instanceof Error ? err : new Error(String(err));
      this.state.error = errorObj;
      this.state.isConnecting = false;
      this.notify();
      throw errorObj;
    }
  }

  public disconnect(): void {
    this.state = {
      publicKey: null,
      network: null,
      networkPassphrase: null,
      isConnected: false,
      isConnecting: false,
      error: null,
    };
    this.notify();
  }

  public async signTransaction(xdr: string, options?: FreighterSignOptions): Promise<string> {
    if (!this.state.isConnected || !this.state.publicKey) {
      throw new FreighterWalletError("Wallet is not connected. Call connect() first.", "NOT_CONNECTED");
    }

    let activeApi = this.api;
    if (!activeApi && typeof window !== "undefined") {
      activeApi = (window as any).freighter;
    }

    return signWithFreighter(xdr, {
      ...options,
      accountToSign: options?.accountToSign || this.state.publicKey,
    }, activeApi);
  }
}
