import { MockSorobanRpcServer } from "../src/mockRpcServer";

describe("MockSorobanRpcServer", () => {
  it("should return health status and initial ledger sequence", async () => {
    const server = new MockSorobanRpcServer({ latestLedgerSequence: 54321 });

    const health = await server.handleRequest("getHealth");
    expect(health.status).toBe("healthy");

    const ledger = await server.handleRequest("getLatestLedger");
    expect(ledger.sequence).toBe(54321);
    expect(ledger.protocolVersion).toBe(20);
  });

  it("should simulate transactions and return cost metrics", async () => {
    const server = new MockSorobanRpcServer({
      simulateCost: { cpuInsns: "99999", memBytes: "4444" },
    });

    const sim = await server.handleRequest("simulateTransaction", {
      transaction: "BASE64XDR",
    });

    expect(sim.minResourceFee).toBe("150");
    expect(sim.cost.cpuInsns).toBe("99999");
    expect(sim.results).toHaveLength(1);
  });

  it("should handle sendTransaction, getTransaction, and advance ledgers", async () => {
    const server = new MockSorobanRpcServer({ latestLedgerSequence: 100 });

    const sendRes = await server.handleRequest("sendTransaction", {
      transaction: "BASE64XDR",
    });
    expect(sendRes.status).toBe("PENDING");
    expect(sendRes.hash).toContain("tx-hash-");

    const txRes = await server.handleRequest("getTransaction", {
      hash: sendRes.hash,
    });
    expect(txRes.status).toBe("SUCCESS");

    const nextLedger = server.advanceLedger(5);
    expect(nextLedger).toBe(105);

    const log = server.getRequestLog();
    expect(log).toHaveLength(2);
    expect(log[0].method).toBe("sendTransaction");
    expect(log[1].method).toBe("getTransaction");

    server.clearRequestLog();
    expect(server.getRequestLog()).toHaveLength(0);
  });

  it("should throw for unsupported RPC methods", async () => {
    const server = new MockSorobanRpcServer();
    await expect(server.handleRequest("unsupportedRpcMethod")).rejects.toThrow(
      /Unsupported Mock Soroban RPC method/
    );
  });
});
