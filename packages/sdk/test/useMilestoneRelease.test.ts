import { MilestoneReleaseManager } from "../src/hooks/useMilestoneRelease";
import { EscrowClient } from "../src/client";
import { EscrowStatus } from "../src/types";

describe("MilestoneReleaseManager", () => {
  const dummyClient = new EscrowClient({
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
    contractId: "CDUMMYCONTRACTID1234567890123456789012345678901234567890",
  });

  const mockEscrow = {
    jobId: "job-release-1",
    client: "GCLIENT_ADDR",
    freelancer: "GFREELANCER_ADDR",
    token: "GTOKEN_ADDR",
    totalAmount: 10000000n,
    releasedAmount: 3000000n,
    status: EscrowStatus.InProgress,
    createdAt: 1000,
    timeoutTimestamp: 2000,
    arbitrator: "GARB_ADDR",
  };

  const pendingMilestone = {
    id: 2,
    description: "Milestone 2",
    percentage: 30,
    amount: 3000000n,
    approved: false,
  };

  const approvedMilestone = {
    id: 1,
    description: "Milestone 1",
    percentage: 30,
    amount: 3000000n,
    approved: true,
  };

  it("calculates milestone release summary and balance projection accurately", () => {
    const manager = new MilestoneReleaseManager(dummyClient, "job-release-1");
    const summary = manager.calculateSummary(pendingMilestone, mockEscrow);

    expect(summary.milestoneId).toBe(2);
    expect(summary.amount).toBe(3000000n);
    expect(summary.newReleasedAmount).toBe(6000000n);
    expect(summary.newRemainingAmount).toBe(4000000n);
  });

  it("rejects already approved milestone", () => {
    const manager = new MilestoneReleaseManager(dummyClient, "job-release-1");
    expect(() => manager.calculateSummary(approvedMilestone, mockEscrow)).toThrow(
      "already been approved"
    );
  });

  it("rejects release exceeding total escrow amount", () => {
    const manager = new MilestoneReleaseManager(dummyClient, "job-release-1");
    const excessiveMilestone = {
      id: 3,
      description: "Huge Milestone",
      percentage: 80,
      amount: 8000000n,
      approved: false,
    };
    expect(() => manager.calculateSummary(excessiveMilestone, mockEscrow)).toThrow(
      "exceed escrow total amount"
    );
  });

  it("validates input arguments before release execution", async () => {
    const manager = new MilestoneReleaseManager(dummyClient, "job-release-1");
    const mockExecutor = jest.fn();

    await expect(manager.releaseMilestone(0, "GCLIENT", mockExecutor)).rejects.toThrow(
      "Valid positive milestone ID is required"
    );
    await expect(manager.releaseMilestone(1, "", mockExecutor)).rejects.toThrow(
      "Client address is required"
    );
  });

  it("submits release successfully and updates transaction state", async () => {
    const manager = new MilestoneReleaseManager(dummyClient, "job-release-1");
    const notifications: any[] = [];
    manager.subscribe((s) => notifications.push(s));

    const mockExecutor = jest.fn().mockResolvedValue({ txHash: "0xHASH12345" });
    const txHash = await manager.releaseMilestone(2, "GCLIENT_ADDR", mockExecutor);

    expect(txHash).toBe("0xHASH12345");
    expect(mockExecutor).toHaveBeenCalledWith("job-release-1", 2, "GCLIENT_ADDR");

    const finalState = manager.getState();
    expect(finalState.isSubmitting).toBe(false);
    expect(finalState.txHash).toBe("0xHASH12345");
    expect(finalState.error).toBeNull();
  });

  it("captures execution error and updates state", async () => {
    const manager = new MilestoneReleaseManager(dummyClient, "job-release-1");
    const mockExecutor = jest.fn().mockRejectedValue(new Error("RPC submission timeout"));

    await expect(manager.releaseMilestone(2, "GCLIENT_ADDR", mockExecutor)).rejects.toThrow(
      "RPC submission timeout"
    );

    const state = manager.getState();
    expect(state.isSubmitting).toBe(false);
    expect(state.error?.message).toBe("RPC submission timeout");
  });
});
