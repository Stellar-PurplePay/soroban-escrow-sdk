import { DisputeResolutionManager } from "../src/hooks/useDisputeResolution";
import { EscrowClient } from "../src/client";

describe("DisputeResolutionManager", () => {
  const dummyClient = new EscrowClient({
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
    contractId: "CDUMMYCONTRACTID1234567890123456789012345678901234567890",
  });

  it("calculates dispute split conserving 100% of remaining funds", () => {
    const manager = new DisputeResolutionManager(dummyClient, "job-dispute-1");
    const remaining = 10000000n;

    // 50% split
    const split50 = manager.calculateSplit(50, remaining);
    expect(split50.clientAmount).toBe(5000000n);
    expect(split50.freelancerAmount).toBe(5000000n);
    expect(split50.clientAmount + split50.freelancerAmount).toBe(remaining);

    // 70% client, 30% freelancer
    const split70 = manager.calculateSplit(70, remaining);
    expect(split70.clientAmount).toBe(7000000n);
    expect(split70.freelancerAmount).toBe(3000000n);
    expect(split70.clientAmount + split70.freelancerAmount).toBe(remaining);

    // 100% client refund
    const split100 = manager.calculateSplit(100, remaining);
    expect(split100.clientAmount).toBe(10000000n);
    expect(split100.freelancerAmount).toBe(0n);
    expect(split100.clientAmount + split100.freelancerAmount).toBe(remaining);
  });

  it("rejects invalid split percentages and non-positive remaining funds", () => {
    const manager = new DisputeResolutionManager(dummyClient, "job-dispute-1");
    expect(() => manager.calculateSplit(-5, 1000n)).toThrow("between 0 and 100");
    expect(() => manager.calculateSplit(105, 1000n)).toThrow("between 0 and 100");
    expect(() => manager.calculateSplit(50, 0n)).toThrow("Remaining amount must be positive");
  });

  it("sets and validates evidence hash", () => {
    const manager = new DisputeResolutionManager(dummyClient, "job-dispute-1");
    manager.setEvidenceHash("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi");
    expect(manager.getState().evidenceHash).toBe(
      "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
    );

    expect(() => manager.setEvidenceHash("   ")).toThrow("Evidence hash must be non-empty");
  });

  it("submits dispute initiation successfully", async () => {
    const manager = new DisputeResolutionManager(dummyClient, "job-dispute-1");
    const mockExecutor = jest.fn().mockResolvedValue({ txHash: "0xDISPUTE_TX" });

    const tx = await manager.raiseDispute(
      "GCLIENT_ADDR",
      "ipfs://QmEvidenceHash",
      mockExecutor
    );
    expect(tx).toBe("0xDISPUTE_TX");
    expect(mockExecutor).toHaveBeenCalledWith(
      "job-dispute-1",
      "GCLIENT_ADDR",
      "ipfs://QmEvidenceHash"
    );

    const state = manager.getState();
    expect(state.isSubmitting).toBe(false);
    expect(state.txHash).toBe("0xDISPUTE_TX");
    expect(state.error).toBeNull();
  });

  it("resolves dispute with arbitrator split execution", async () => {
    const manager = new DisputeResolutionManager(dummyClient, "job-dispute-1");
    const mockExecutor = jest.fn().mockResolvedValue({ txHash: "0xRESOLVE_TX" });

    const tx = await manager.resolveDispute(
      "GARBITRATOR_ADDR",
      6000000n,
      4000000n,
      mockExecutor
    );
    expect(tx).toBe("0xRESOLVE_TX");
    expect(mockExecutor).toHaveBeenCalledWith(
      "job-dispute-1",
      "GARBITRATOR_ADDR",
      6000000n,
      4000000n
    );
  });

  it("captures resolution executor errors", async () => {
    const manager = new DisputeResolutionManager(dummyClient, "job-dispute-1");
    const mockExecutor = jest.fn().mockRejectedValue(new Error("Arbitrator signature invalid"));

    await expect(
      manager.resolveDispute("GARBITRATOR_ADDR", 500n, 500n, mockExecutor)
    ).rejects.toThrow("Arbitrator signature invalid");

    expect(manager.getState().error?.message).toBe("Arbitrator signature invalid");
  });
});
