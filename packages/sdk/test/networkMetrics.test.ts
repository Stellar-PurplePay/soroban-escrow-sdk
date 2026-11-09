import { NetworkMetricsCollector } from "../src/networkMetrics";

describe("NetworkMetricsCollector", () => {
  const mockRpcUrl = "https://soroban-testnet.stellar.org";

  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("classifies congestion levels accurately", () => {
    const collector = new NetworkMetricsCollector({ rpcUrl: mockRpcUrl });

    expect(collector.classifyCongestion(1.0, 500)).toBe("OPTIMAL");
    expect(collector.classifyCongestion(1.5, 800)).toBe("MODERATE");
    expect(collector.classifyCongestion(1.0, 1800)).toBe("MODERATE");
    expect(collector.classifyCongestion(2.8, 800)).toBe("CONGESTED");
    expect(collector.classifyCongestion(1.0, 3600)).toBe("CONGESTED");
  });

  it("samples network metrics from mock RPC fetch", async () => {
    const collector = new NetworkMetricsCollector({ rpcUrl: mockRpcUrl, defaultBaseFee: 100 });
    const mockFetch = jest.fn().mockResolvedValue({
      json: jest.fn().mockResolvedValue({
        result: { sequence: 504200 },
      }),
    });

    const snapshot = await collector.sampleMetrics(mockFetch);

    expect(snapshot.baseFeeStroops).toBe(100);
    expect(snapshot.ledgerSequence).toBe(504200);
    expect(snapshot.status).toBe("OPTIMAL");
    expect(snapshot.lastChecked).toBeGreaterThan(0);
    expect(mockFetch).toHaveBeenCalledWith(
      mockRpcUrl,
      expect.objectContaining({ method: "POST" })
    );
  });

  it("handles RPC network error and returns error state", async () => {
    const collector = new NetworkMetricsCollector({ rpcUrl: mockRpcUrl });
    const mockFetch = jest.fn().mockRejectedValue(new Error("RPC Timeout 504"));

    const snapshot = await collector.sampleMetrics(mockFetch);

    expect(snapshot.error).toBe("RPC Timeout 504");
    expect(snapshot.feeMultiplier).toBe(2.0);
    expect(snapshot.status).toBe("MODERATE");
  });

  it("manages monitoring timer lifecycle", async () => {
    const collector = new NetworkMetricsCollector({ rpcUrl: mockRpcUrl });
    const mockFetch = jest.fn().mockResolvedValue({
      json: jest.fn().mockResolvedValue({ result: { sequence: 100 } }),
    });
    const mockCallback = jest.fn();

    expect(collector.isMonitoring()).toBe(false);
    collector.startMonitoring(2000, mockCallback, mockFetch);
    expect(collector.isMonitoring()).toBe(true);

    for (let i = 0; i < 5; i++) await Promise.resolve();
    expect(mockCallback).toHaveBeenCalledTimes(1);

    jest.advanceTimersByTime(2000);
    for (let i = 0; i < 5; i++) await Promise.resolve();
    expect(mockCallback).toHaveBeenCalledTimes(2);

    collector.stopMonitoring();
    expect(collector.isMonitoring()).toBe(false);

    jest.advanceTimersByTime(4000);
    for (let i = 0; i < 5; i++) await Promise.resolve();
    expect(mockCallback).toHaveBeenCalledTimes(2);
  });
});
