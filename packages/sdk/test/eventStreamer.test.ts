import { ContractEventStreamer } from "../src/eventStreamer";

describe("ContractEventStreamer", () => {
  const mockOptions = {
    rpcUrl: "https://soroban-testnet.stellar.org",
    contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
    pollIntervalMs: 2000,
    initialBackoffMs: 1000,
    maxBackoffMs: 8000,
  };

  beforeEach(() => {
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("subscribes and receives typed and wildcard events", async () => {
    const streamer = new ContractEventStreamer(mockOptions);
    const releaseEvents: any[] = [];
    const allEvents: any[] = [];

    const unsubRelease = streamer.on("released", (e) => releaseEvents.push(e));
    const unsubWildcard = streamer.on("*", (e) => allEvents.push(e));

    const mockFetch = jest.fn().mockResolvedValue({
      json: jest.fn().mockResolvedValue({
        result: {
          events: [
            {
              id: "evt-1",
              topic: ["escrow", "released"],
              data: { amount: 500 },
              ledger: 504001,
            },
            {
              id: "evt-2",
              topic: ["escrow", "disputed"],
              data: { reason: "scope" },
              ledger: 504002,
            },
          ],
        },
      }),
    });

    const count = await streamer.pollEvents(mockFetch);
    expect(count).toBe(2);
    expect(releaseEvents).toHaveLength(1);
    expect(releaseEvents[0].id).toBe("evt-1");
    expect(allEvents).toHaveLength(2);
    expect(streamer.getLastLedger()).toBe(504002);

    // Test unsubscribe
    unsubRelease();
    unsubWildcard();

    await streamer.pollEvents(mockFetch);
    expect(releaseEvents).toHaveLength(1);
    expect(allEvents).toHaveLength(2);
  });

  it("applies exponential backoff on fetch errors", async () => {
    const streamer = new ContractEventStreamer(mockOptions);
    const mockErrorFetch = jest.fn().mockRejectedValue(new Error("RPC Unavailable"));

    expect(streamer.getReconnectAttempts()).toBe(0);

    await expect(streamer.pollEvents(mockErrorFetch)).rejects.toThrow("RPC Unavailable");
    expect(streamer.getReconnectAttempts()).toBe(1);

    await expect(streamer.pollEvents(mockErrorFetch)).rejects.toThrow("RPC Unavailable");
    expect(streamer.getReconnectAttempts()).toBe(2);
  });

  it("manages start and stop lifecycle", async () => {
    const streamer = new ContractEventStreamer(mockOptions);
    const mockFetch = jest.fn().mockResolvedValue({
      json: jest.fn().mockResolvedValue({ result: { events: [] } }),
    });

    expect(streamer.getIsRunning()).toBe(false);
    streamer.start(mockFetch);
    expect(streamer.getIsRunning()).toBe(true);

    streamer.stop();
    expect(streamer.getIsRunning()).toBe(false);
  });
});
