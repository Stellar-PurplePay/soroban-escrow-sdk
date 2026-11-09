import { EscrowStateManager } from "../src/hooks/useEscrowState";
import { EscrowClient } from "../src/client";
import { EscrowStatus } from "../src/types";

describe("EscrowStateManager", () => {
  const dummyClient = new EscrowClient({
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
    contractId: "CDUMMYCONTRACTID1234567890123456789012345678901234567890",
  });

  const mockEscrow = {
    jobId: "job-101",
    client: "GCLIENT...",
    freelancer: "GFREELANCER...",
    token: "GTOKEN...",
    totalAmount: 1000000n,
    releasedAmount: 0n,
    status: EscrowStatus.InProgress,
    createdAt: 1000,
    timeoutTimestamp: 2000,
    arbitrator: "GARB...",
  };

  const mockProgress = {
    totalAmount: 1000000n,
    releasedAmount: 0n,
    percentageReleased: 0,
    status: EscrowStatus.InProgress,
  };

  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("subscribes and notifies listener of state transitions", async () => {
    const manager = new EscrowStateManager(dummyClient, "job-101");
    const notifications: any[] = [];

    const unsubscribe = manager.subscribe((state) => {
      notifications.push(state);
    });

    expect(notifications).toHaveLength(1);
    expect(notifications[0].isLoading).toBe(false);
    expect(notifications[0].escrow).toBeNull();

    const mockFetcher = jest.fn().mockResolvedValue({
      escrow: mockEscrow,
      milestones: [{ id: 1, description: "M1", percentage: 100, amount: 1000000n, approved: false }],
      progress: mockProgress,
      ledgerSequence: 504200,
    });

    await manager.refresh(mockFetcher);

    expect(notifications.length).toBeGreaterThanOrEqual(3);
    const finalState = manager.getState();
    expect(finalState.isLoading).toBe(false);
    expect(finalState.escrow?.jobId).toBe("job-101");
    expect(finalState.milestones).toHaveLength(1);
    expect(finalState.ledgerSequence).toBe(504200);
    expect(finalState.error).toBeNull();

    unsubscribe();
  });

  it("captures fetch errors and notifies listeners", async () => {
    const manager = new EscrowStateManager(dummyClient, "job-error");
    const mockFetcher = jest.fn().mockRejectedValue(new Error("RPC network failure"));

    await manager.refresh(mockFetcher);

    const state = manager.getState();
    expect(state.isLoading).toBe(false);
    expect(state.error?.message).toBe("RPC network failure");
    expect(state.escrow).toBeNull();
  });

  it("handles auto-polling lifecycle and timer cleanup", async () => {
    const manager = new EscrowStateManager(dummyClient, "job-poll");
    const mockFetcher = jest.fn().mockResolvedValue({
      escrow: mockEscrow,
      milestones: [],
      progress: mockProgress,
    });

    expect(manager.isPolling()).toBe(false);
    manager.startPolling(1000, mockFetcher);
    expect(manager.isPolling()).toBe(true);
    await Promise.resolve();
    expect(mockFetcher).toHaveBeenCalledTimes(1);

    jest.advanceTimersByTime(1000);
    await Promise.resolve();
    expect(mockFetcher).toHaveBeenCalledTimes(2);

    jest.advanceTimersByTime(1000);
    await Promise.resolve();
    expect(mockFetcher).toHaveBeenCalledTimes(3);

    manager.stopPolling();
    expect(manager.isPolling()).toBe(false);

    jest.advanceTimersByTime(2000);
    await Promise.resolve();
    expect(mockFetcher).toHaveBeenCalledTimes(3);
  });

  it("applies optimistic updates and supports rollback", async () => {
    const manager = new EscrowStateManager(dummyClient, "job-optimistic");
    const mockFetcher = jest.fn().mockResolvedValue({
      escrow: mockEscrow,
      milestones: [],
      progress: mockProgress,
    });

    await manager.refresh(mockFetcher);
    expect(manager.getState().escrow?.status).toBe(EscrowStatus.InProgress);

    // Apply optimistic update
    manager.applyOptimisticUpdate({ status: EscrowStatus.Completed, releasedAmount: 1000000n });
    expect(manager.getState().escrow?.status).toBe(EscrowStatus.Completed);
    expect(manager.getState().escrow?.releasedAmount).toBe(1000000n);

    // Rollback
    manager.rollbackOptimisticUpdate();
    expect(manager.getState().escrow?.status).toBe(EscrowStatus.InProgress);
    expect(manager.getState().escrow?.releasedAmount).toBe(0n);
  });

  it("cleans up resources on destroy", () => {
    const manager = new EscrowStateManager(dummyClient, "job-destroy");
    const mockFetcher = jest.fn().mockResolvedValue({
      escrow: mockEscrow,
      milestones: [],
      progress: mockProgress,
    });

    manager.startPolling(1000, mockFetcher);
    expect(manager.isPolling()).toBe(true);

    manager.destroy();
    expect(manager.isPolling()).toBe(false);
  });
});
