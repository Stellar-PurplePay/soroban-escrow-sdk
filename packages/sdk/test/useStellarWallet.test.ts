import { StellarWalletManager } from "../src/hooks/useStellarWallet";
import { FreighterApi } from "../src/freighter";

describe("StellarWalletManager", () => {
  const mockFreighterApi: FreighterApi = {
    isConnected: jest.fn().mockResolvedValue(true),
    isAllowed: jest.fn().mockResolvedValue(true),
    setAllowed: jest.fn().mockResolvedValue(true),
    getUserInfo: jest.fn().mockResolvedValue({
      publicKey: "GBBD6W7J4T6J47X6C6Q3S7D4O445X4J7G4Y6B6V4W7J4T6J47X6C6Q3S",
    }),
    getNetworkDetails: jest.fn().mockResolvedValue({
      network: "TESTNET",
      networkPassphrase: "Test SDF Network ; September 2015",
    }),
    signTransaction: jest.fn().mockResolvedValue("AAAA_SIGNED_XDR_RESULT"),
  };

  it("initializes with disconnected state", () => {
    const manager = new StellarWalletManager();
    const state = manager.getState();
    expect(state.isConnected).toBe(false);
    expect(state.publicKey).toBeNull();
    expect(state.network).toBeNull();
    expect(state.error).toBeNull();
  });

  it("successfully connects and updates state with public key and network", async () => {
    const manager = new StellarWalletManager(mockFreighterApi);
    const notifications: any[] = [];
    const unsubscribe = manager.subscribe((s) => notifications.push(s));

    const pubKey = await manager.connect();
    expect(pubKey).toBe("GBBD6W7J4T6J47X6C6Q3S7D4O445X4J7G4Y6B6V4W7J4T6J47X6C6Q3S");

    const finalState = manager.getState();
    expect(finalState.isConnected).toBe(true);
    expect(finalState.publicKey).toBe("GBBD6W7J4T6J47X6C6Q3S7D4O445X4J7G4Y6B6V4W7J4T6J47X6C6Q3S");
    expect(finalState.network).toBe("TESTNET");
    expect(finalState.networkPassphrase).toBe("Test SDF Network ; September 2015");
    expect(finalState.error).toBeNull();

    unsubscribe();
  });

  it("handles missing Freighter provider gracefully", async () => {
    const manager = new StellarWalletManager();
    await expect(manager.connect()).rejects.toThrow("Freighter extension is not installed");
    expect(manager.getState().error?.message).toContain("not installed");
    expect(manager.getState().isConnected).toBe(false);
  });

  it("handles locked Freighter wallet error", async () => {
    const lockedApi: FreighterApi = {
      ...mockFreighterApi,
      isConnected: jest.fn().mockResolvedValue(false),
    };
    const manager = new StellarWalletManager(lockedApi);
    await expect(manager.connect()).rejects.toThrow("Freighter wallet is locked");
    expect(manager.getState().isConnected).toBe(false);
  });

  it("signs transaction when connected and passes accountToSign", async () => {
    const manager = new StellarWalletManager(mockFreighterApi);
    await manager.connect();

    const signedXdr = await manager.signTransaction("AAAA_RAW_XDR");
    expect(signedXdr).toBe("AAAA_SIGNED_XDR_RESULT");
    expect(mockFreighterApi.signTransaction).toHaveBeenCalledWith(
      "AAAA_RAW_XDR",
      expect.objectContaining({
        accountToSign: "GBBD6W7J4T6J47X6C6Q3S7D4O445X4J7G4Y6B6V4W7J4T6J47X6C6Q3S",
      })
    );
  });

  it("throws error when trying to sign without connecting", async () => {
    const manager = new StellarWalletManager(mockFreighterApi);
    await expect(manager.signTransaction("AAAA_RAW_XDR")).rejects.toThrow("Wallet is not connected");
  });

  it("resets state on disconnect", async () => {
    const manager = new StellarWalletManager(mockFreighterApi);
    await manager.connect();
    expect(manager.getState().isConnected).toBe(true);

    manager.disconnect();
    const state = manager.getState();
    expect(state.isConnected).toBe(false);
    expect(state.publicKey).toBeNull();
    expect(state.network).toBeNull();
  });
});
