import {
  NetworkMetricsCollector,
  formatStellarAddress,
  toAccessibleAddress,
  ContractEventStreamer,
} from "../src";

describe("SDK Network Metrics & Event Streamer Barrel Export", () => {
  it("exports NetworkMetricsCollector and initializes", () => {
    expect(NetworkMetricsCollector).toBeDefined();
    const collector = new NetworkMetricsCollector({
      rpcUrl: "https://soroban-testnet.stellar.org",
    });
    expect(collector.classifyCongestion(1.0, 500)).toBe("OPTIMAL");
  });

  it("exports formatStellarAddress and toAccessibleAddress", () => {
    expect(formatStellarAddress).toBeDefined();
    expect(toAccessibleAddress).toBeDefined();

    const formatted = formatStellarAddress(
      "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
    );
    expect(formatted).toBe("GBBD47...LLFLA5");
  });

  it("exports ContractEventStreamer and initializes", () => {
    expect(ContractEventStreamer).toBeDefined();
    const streamer = new ContractEventStreamer({
      rpcUrl: "https://soroban-testnet.stellar.org",
      contractId: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
    });
    expect(streamer.getIsRunning()).toBe(false);
  });
});
