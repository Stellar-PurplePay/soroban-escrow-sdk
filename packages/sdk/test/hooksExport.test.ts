import {
  EscrowStateManager,
  StellarWalletManager,
  MilestoneReleaseManager,
  DisputeResolutionManager,
  EscrowClient,
} from "../src";

describe("SDK Hooks Barrel Export", () => {
  const dummyClient = new EscrowClient({
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
    contractId: "CDUMMYCONTRACTID1234567890123456789012345678901234567890",
  });

  it("exports EscrowStateManager and instantiates cleanly", () => {
    expect(EscrowStateManager).toBeDefined();
    const manager = new EscrowStateManager(dummyClient, "job-export-test");
    expect(manager.getState().escrow).toBeNull();
  });

  it("exports StellarWalletManager and instantiates cleanly", () => {
    expect(StellarWalletManager).toBeDefined();
    const manager = new StellarWalletManager();
    expect(manager.getState().isConnected).toBe(false);
  });

  it("exports MilestoneReleaseManager and instantiates cleanly", () => {
    expect(MilestoneReleaseManager).toBeDefined();
    const manager = new MilestoneReleaseManager(dummyClient, "job-export-test");
    expect(manager.getState().isSubmitting).toBe(false);
  });

  it("exports DisputeResolutionManager and instantiates cleanly", () => {
    expect(DisputeResolutionManager).toBeDefined();
    const manager = new DisputeResolutionManager(dummyClient, "job-export-test");
    expect(manager.getState().jobId).toBe("job-export-test");
  });
});
